use std::collections::{HashMap, HashSet};

mod game;
use game::*;

const SGR_RESET: &str = "\x1b[0m";
const SGR_RED: &str = "\x1b[31m";
const SGR_GREEN: &str = "\x1b[32m";
const SGR_BLUE: &str = "\x1b[34m";

enum Color {
    Red,
    Green,
    Blue,
}

impl Color {
    fn sgr(&self) -> &str {
        match self {
            Color::Red => SGR_RED,
            Color::Green => SGR_GREEN,
            Color::Blue => SGR_BLUE,
        }
    }
}

fn main() {
    #[rustfmt::skip]
    let board = Board::new(
        7,
        concat!(
            "RADOGAA",
            "REWSONO",
            "HENLITH",
            "LPMNINE",
            "BAIESER",
            "ORNBRTA",
            "PEUUSCC",
            "NCTRYOR",
            "AURNADE",
            "WFETUDS",
            "ULPRAAF",
            "AIDLELY",
        ),
    );
    let ruleset = Ruleset {
        min_length: 3,
        max_length: 8,
    };
    let mut game = Game::<Color>::new(&board, &ruleset);

    let mut region = Region::new();
    region.add_square((0, 0).into());
    region.add_square((1, 0).into());
    region.add_square((1, 1).into());
    region.add_square((0, 1).into());

    let checked_region = game.check_region(&region).unwrap();
    game.add_region(checked_region, Color::Red);

    let mut region = Region::new();
    region.add_square((6, 0).into());
    region.add_square((5, 1).into());
    region.add_square((6, 1).into());
    region.add_square((5, 2).into());
    region.add_square((6, 2).into());
    region.add_square((6, 3).into());
    region.add_square((6, 4).into());

    let checked_region = game.check_region(&region).unwrap();
    game.add_region(checked_region, Color::Green);

    let mut region = Region::new();
    region.add_square((3, 3).into());
    region.add_square((4, 3).into());
    region.add_square((5, 3).into());
    region.add_square((5, 4).into());

    let checked_region = game.check_region(&region).unwrap();
    game.add_region(checked_region, Color::Blue);

    let square_to_color = game
        .regions()
        .flat_map(|(region, data)| region.squares().map(move |square| (square, data)))
        .collect::<HashMap<_, _>>();

    for y in 0..board.height() {
        for x in 0..board.width() {
            let square = (x, y).into();

            match square_to_color.get(&square) {
                Some(color) => {
                    let sgr = color.sgr();
                    print!("{sgr}");
                }
                None => print!("{SGR_RESET}"),
            }

            let c = board.get(square);

            print!("{c}");
        }
        println!();
    }
}
