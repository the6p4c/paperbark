use chrono::{TimeZone, Utc};
use itertools::izip;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::io;
use std::iter;
use std::path::Path;
use structopt::StructOpt;
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

struct OfficialData {
    dictionary_data: DictionaryData,
    game_data: GameData,
}

impl OfficialData {
    fn from_paths<P1: AsRef<Path>, P2: AsRef<Path>>(
        dictionary_path: P1,
        game_path: P2,
    ) -> io::Result<Self> {
        let dictionary_json = fs::read_to_string(dictionary_path)?;
        let game_json = fs::read_to_string(game_path)?;

        Ok(Self::from_json(&dictionary_json, &game_json))
    }

    fn from_web_today() -> reqwest::Result<Self> {
        let epoch = Utc.ymd(2022, 05, 06).and_hms(0, 0, 0);
        let puzzle_id = Utc::now().signed_duration_since(epoch).num_days() + 1;

        Self::from_web(puzzle_id)
    }

    fn from_web(puzzle_id: i64) -> reqwest::Result<Self> {
        const BASE_URL: &str = "https://www.andrewt.net/puzzles/cell-tower";

        let client = reqwest::blocking::Client::new();
        let dictionary_json = client
            .get(format!("{BASE_URL}/assets/words.json"))
            .send()?
            .text()?;
        let game_json = client
            .get(format!("{BASE_URL}/puzzles/{puzzle_id}.json"))
            .send()?
            .text()?;

        Ok(Self::from_json(&dictionary_json, &game_json))
    }

    fn from_json(dictionary_json: &str, game_json: &str) -> Self {
        let dictionary_data = serde_json::from_str(dictionary_json).unwrap();
        let game_data = serde_json::from_str(game_json).unwrap();

        Self {
            dictionary_data,
            game_data,
        }
    }

    fn board(&self) -> Board {
        let GameData {
            width,
            height,
            regions,
            words,
            ..
        } = &self.game_data;

        let empty_chars = iter::repeat(None).take(width * height).collect::<Vec<_>>();

        assert_eq!(regions.len(), words.len());
        let chars = izip!(words, regions).fold(empty_chars, |mut chars, (word, region)| {
            assert_eq!(word.len(), region.len());

            for (c, (x, y)) in izip!(word.chars(), region) {
                let char_slot = &mut chars[y * width + x];
                assert_eq!(*char_slot, None);
                *char_slot = Some(c.to_uppercase().next().unwrap());
            }

            chars
        });

        let chars = chars.into_iter().map(Option::unwrap).collect::<String>();

        Board::new(*width, chars)
    }

    fn ruleset(&self) -> Ruleset {
        let dictionary = self
            .dictionary_data
            .0
            .iter()
            .map(|w| w.to_uppercase())
            .collect();

        Ruleset {
            min_length: self.game_data.min_size,
            max_length: self.game_data.max_size,
            dictionary,
        }
    }
}

#[derive(StructOpt)]
#[structopt(about = "a terminal-based clone of the cell tower puzzle game")]
enum Paperbark {
    Today,
    Day { puzzle_id: u64 },
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Paperbark::from_args();
    match opt {
        Paperbark::Today => {
            let official_data = OfficialData::from_web_today()?;
            let board = official_data.board();
            let ruleset = official_data.ruleset();

            let game = Game::<Color>::new(&board, &ruleset);
            ui::run(game)?;
        }
        Paperbark::Day { puzzle_id } => {
            let official_data = OfficialData::from_web(puzzle_id as i64)?;
            let board = official_data.board();
            let ruleset = official_data.ruleset();

            let game = Game::<Color>::new(&board, &ruleset);
            ui::run(game)?;
        }
    }

    Ok(())
}
