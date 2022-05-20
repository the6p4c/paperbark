use itertools::izip;
use serde::Deserialize;
use std::fs::File;
use std::io::{self, Read};
use std::iter;
use tui::style::Color;

mod game;
use game::*;

mod ui;

#[derive(Deserialize, Debug)]
struct DictionaryData(Vec<String>);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GameData {
    width: usize,
    height: usize,
    min_size: usize,
    max_size: usize,
    regions: Vec<Vec<(usize, usize)>>,
    words: Vec<String>,
}

fn main() -> io::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let dictionary_data: DictionaryData = {
        let mut file = File::open(&args[1])?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        serde_json::from_str(&contents).unwrap()
    };
    let game_data: GameData = {
        let mut file = File::open(&args[2])?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        serde_json::from_str(&contents).unwrap()
    };

    let empty_chars = iter::repeat(None)
        .take(game_data.width * game_data.height)
        .collect::<Vec<_>>();

    assert_eq!(game_data.regions.len(), game_data.words.len());
    let chars =
        izip!(game_data.words, game_data.regions).fold(empty_chars, |mut chars, (word, region)| {
            assert_eq!(word.len(), region.len());

            for (c, (x, y)) in izip!(word.chars(), region) {
                let char_slot = &mut chars[y * game_data.width + x];
                assert_eq!(*char_slot, None);
                *char_slot = Some(c.to_uppercase().next().unwrap());
            }

            chars
        });

    let chars = chars.into_iter().map(Option::unwrap).collect::<String>();
    let board = Board::new(game_data.width, chars);

    let dictionary = dictionary_data
        .0
        .into_iter()
        .map(|w| w.to_uppercase())
        .collect();
    let ruleset = Ruleset {
        min_length: game_data.min_size,
        max_length: game_data.max_size,
        dictionary,
    };
    let game = Game::<Color>::new(&board, &ruleset);

    ui::run(game)
}
