use itertools::iproduct;
use std::collections::HashSet;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Square {
    pub x: usize,
    pub y: usize,
}

impl Square {
    fn is_neighbour_of(&self, other: Square) -> bool {
        let delta = |a: usize, b: usize| a.max(b) - a.min(b);

        let is_horizontal = self.y == other.y;
        let dx = delta(self.x, other.x);

        let is_vertical = self.x == other.x;
        let dy = delta(self.y, other.y);

        (is_horizontal && dx == 1) || (is_vertical && dy == 1)
    }
}

impl From<(usize, usize)> for Square {
    fn from(s: (usize, usize)) -> Self {
        Self { x: s.0, y: s.1 }
    }
}

pub struct Board {
    width: usize,
    height: usize,
    board: Vec<char>,
}

impl Board {
    pub fn new(width: usize, board: impl Into<String>) -> Self {
        let board = board.into();

        assert_eq!(board.len() % width, 0);
        let height = board.len() / width;

        let board = board.chars().collect();

        Self {
            width,
            height,
            board,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn get(&self, s: Square) -> char {
        self.board[s.y * self.width + s.x]
    }
}

#[derive(Clone, PartialEq)]
pub struct Region(HashSet<Square>);

impl Region {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn add_square(&mut self, square: Square) -> bool {
        self.0.insert(square)
    }

    pub fn remove_square(&mut self, square: Square) -> bool {
        self.0.remove(&square)
    }

    pub fn squares(&self) -> impl Iterator<Item = Square> + '_ {
        self.0.iter().copied()
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }

    pub fn word(&self, board: &Board) -> String {
        let mut squares = self.0.iter().copied().collect::<Vec<_>>();
        squares.sort_unstable_by(|a, b| {
            let x_ordering = a.x.cmp(&b.x);
            let y_ordering = a.y.cmp(&b.y);

            // top-to-bottom (y first), left-to-right (x second)
            y_ordering.then(x_ordering)
        });

        squares.into_iter().map(|s| board.get(s)).collect()
    }

    fn is_in_bounds(&self, board: &Board) -> bool {
        let is_out_of_bounds = self
            .0
            .iter()
            .any(|s| s.x >= board.width || s.y >= board.height);

        !is_out_of_bounds
    }

    fn is_contiguous(&self) -> bool {
        if self.size() == 0 {
            return true;
        }

        let mut so_far = HashSet::new();
        let mut remaining = self.0.clone();

        let start = *remaining.iter().next().unwrap();
        so_far.insert(start);
        remaining.remove(&start);

        while !remaining.is_empty() {
            // try and find a square in our remaining set which is adjacent to at least one square
            // in the region we've built up so far
            let adjacent = remaining
                .iter()
                .find(|s1| so_far.iter().any(|s2| s1.is_neighbour_of(*s2)))
                .copied();

            // if we can find one, add it to the region, if not, the region isn't contiguous
            match adjacent {
                Some(s) => {
                    so_far.insert(s);
                    remaining.remove(&s);
                }
                None => return false,
            }
        }

        true
    }
}

pub struct Ruleset {
    pub min_length: usize,
    pub max_length: usize,
    pub dictionary: HashSet<String>,
}

pub struct CheckedRegion<'a>(&'a Region);

#[derive(Debug)]
pub enum CheckRegionError {
    TooShort,
    TooLong,
    OutOfBounds,
    Overlapping,
    NotContiguous,
    NotInDictionary,
}

pub struct Game<'a, D> {
    board: &'a Board,
    ruleset: &'a Ruleset,
    regions: Vec<(Region, D)>,
}

impl<'a, D> Game<'a, D> {
    pub fn new(board: &'a Board, ruleset: &'a Ruleset) -> Self {
        Self {
            board,
            ruleset,
            regions: vec![],
        }
    }

    pub fn is_complete(&self) -> bool {
        let all_squares = iproduct!(0..self.board.width(), 0..self.board.height())
            .map(|s| s.into())
            .collect::<HashSet<_>>();
        let used_squares = self.regions.iter().map(|(region, _)| &region.0).fold(
            HashSet::new(),
            |mut used_squares, region| {
                used_squares.extend(region.iter().copied());
                used_squares
            },
        );

        all_squares.difference(&used_squares).count() == 0
    }

    pub fn check_region<'b>(
        &self,
        region: &'b Region,
    ) -> Result<CheckedRegion<'b>, CheckRegionError> {
        if region.size() < self.ruleset.min_length {
            return Err(CheckRegionError::TooShort);
        }

        if region.size() > self.ruleset.max_length {
            return Err(CheckRegionError::TooLong);
        }

        if !region.is_in_bounds(self.board) {
            return Err(CheckRegionError::OutOfBounds);
        }

        let is_overlapping = self
            .regions
            .iter()
            .flat_map(|(region, _)| region.squares())
            .any(|square| region.0.contains(&square));
        if is_overlapping {
            return Err(CheckRegionError::Overlapping);
        }

        if !region.is_contiguous() {
            return Err(CheckRegionError::NotContiguous);
        }

        let word = region.word(self.board);
        if !self.ruleset.dictionary.contains(&word) {
            return Err(CheckRegionError::NotInDictionary);
        }

        Ok(CheckedRegion(region))
    }

    pub fn add_region(&mut self, region: CheckedRegion, data: D) {
        let region = (*region.0).clone();

        self.regions.push((region, data));
    }

    pub fn remove_region(&mut self, square: Square) -> Option<(Region, D)> {
        let index = self
            .regions
            .iter()
            .position(|(region, _)| region.0.contains(&square));

        match index {
            Some(index) => Some(self.regions.swap_remove(index)),
            None => None,
        }
    }

    pub fn regions(&self) -> impl Iterator<Item = &(Region, D)> {
        self.regions.iter()
    }

    pub fn is_square_free(&self, square: Square) -> bool {
        let is_square_occupied = self
            .regions
            .iter()
            .flat_map(|(region, _)| region.squares())
            .any(|other| square == other);

        !is_square_occupied
    }

    pub fn board(&self) -> &Board {
        self.board
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! region {
        () => {
            Region(HashSet::new())
        };
        ($($square:expr),+ $(,)?) => {
            {
                let mut squares = HashSet::new();
                $(
                    squares.insert($square.into());
                )*
                Region(squares)
            }
        };
    }

    fn board() -> Board {
        #[rustfmt::skip]
        let board = Board::new(
            3,
            concat!(
                "ABC",
                "DEF",
                "GHI",
            )
        );

        board
    }

    #[test]
    fn region_size() {
        let region = region![];
        assert_eq!(region.size(), 0);

        let region = region![(0, 0)];
        assert_eq!(region.size(), 1);

        let region = region![(0, 0), (0, 1)];
        assert_eq!(region.size(), 2);

        let region = region![(0, 0), (0, 1), (1, 0)];
        assert_eq!(region.size(), 3);
    }

    #[test]
    fn region_word() {
        let board = board();

        let region = region![];
        assert_eq!(region.word(&board), "");

        let region = region![(0, 0)];
        assert_eq!(region.word(&board), "A");

        let region = region![(0, 0), (0, 1)];
        assert_eq!(region.word(&board), "AD");

        let region = region![(0, 0), (0, 1), (1, 0)];
        assert_eq!(region.word(&board), "ABD");

        let region = region![(0, 0), (0, 1), (0, 2), (1, 0), (2, 0)];
        assert_eq!(region.word(&board), "ABCDG");

        let region = region![(0, 0), (0, 1), (0, 2), (1, 0), (2, 0), (2, 1), (2, 2),];
        assert_eq!(region.word(&board), "ABCDFGI");
    }

    #[test]
    fn region_is_in_bounds() {
        let board = board();

        let region = region![];
        assert_eq!(region.is_in_bounds(&board), true);

        let region = region![(0, 0)];
        assert_eq!(region.is_in_bounds(&board), true);

        let region = region![(2, 2)];
        assert_eq!(region.is_in_bounds(&board), true);

        let region = region![(3, 2)];
        assert_eq!(region.is_in_bounds(&board), false);

        let region = region![(2, 3)];
        assert_eq!(region.is_in_bounds(&board), false);

        let region = region![(0, 0), (1, 1), (2, 2), (3, 3)];
        assert_eq!(region.is_in_bounds(&board), false);
    }

    #[test]
    fn region_is_contiguous() {
        let board = board();

        let region = region![];
        assert_eq!(region.is_contiguous(), true);

        let region = region![(0, 0)];
        assert_eq!(region.is_contiguous(), true);

        let region = region![(0, 0), (0, 1)];
        assert_eq!(region.is_contiguous(), true);

        let region = region![(0, 0), (1, 0)];
        assert_eq!(region.is_contiguous(), true);

        let region = region![(0, 0), (1, 1)];
        assert_eq!(region.is_contiguous(), false);

        let region = region![(0, 0), (0, 1), (1, 1)];
        assert_eq!(region.is_contiguous(), true);

        let region = region![(0, 0), (1, 0), (1, 1)];
        assert_eq!(region.is_contiguous(), true);

        let region = region![(0, 0), (0, 1), (2, 2)];
        assert_eq!(region.is_contiguous(), false);

        let region = region![(0, 0), (0, 1), (0, 2), (1, 0), (2, 0), (2, 1), (2, 2),];
        assert_eq!(region.is_contiguous(), true);
    }
}
