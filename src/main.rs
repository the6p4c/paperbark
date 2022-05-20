use std::collections::HashSet;
use std::io;
use tui::style::Color;

mod game;
use game::*;

mod ui;

fn main() -> io::Result<()> {
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
        dictionary: HashSet::from(["RARE".to_owned(), "DOG".to_owned()]),
    };
    let mut game = Game::<Color>::new(&board, &ruleset);

    let mut region = Region::new();
    region.add_square((0, 0).into());
    region.add_square((0, 1).into());
    region.add_square((1, 1).into());
    region.add_square((1, 0).into());
    let checked_region = game.check_region(&region).unwrap();
    game.add_region(checked_region, Color::Red);

    ui::run(game)
}
