use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use itertools::{chain, iproduct};
use std::collections::HashMap;
use std::io;
use std::time::Duration;
use tui::backend::Backend;
use tui::backend::CrosstermBackend;
use tui::buffer::Buffer;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Paragraph, Widget};
use tui::Frame;
use tui::Terminal;

use crate::game::*;

enum RegionType {
    Committed(Color),
    Uncommitted,
}

struct GameWidget<'a, 'b, 'c> {
    game: &'a Game<'b, Color>,
    cursor: Square,
    uncommitted: &'c Region,
}

impl GameWidget<'_, '_, '_> {
    fn render_board(&self, area: Rect, buf: &mut Buffer) {
        let GameWidget {
            game,
            cursor,
            uncommitted,
        } = self;

        let game_width = game.board().width();
        let game_height = game.board().height();

        assert!(area.width >= game_width as u16 + 2);
        assert!(area.height >= game_height as u16 + 2);

        let top_left_x = area.x + (area.width - game_width as u16 + 2) / 2;
        let top_left_y = area.y + (area.height - game_height as u16 + 2) / 2;

        let square_to_region_type = game
            .regions()
            .flat_map(|(region, color)| {
                region
                    .squares()
                    .map(move |square| (square, RegionType::Committed(*color)))
            })
            .chain(
                uncommitted
                    .squares()
                    .map(|square| (square, RegionType::Uncommitted)),
            )
            .collect::<HashMap<_, _>>();

        for y in 0..game_height {
            for x in 0..game_width {
                let square = (x, y).into();
                let c = game.board().get(square);
                let region_type = square_to_region_type.get(&square);

                let (fg, bg) = match region_type {
                    Some(RegionType::Committed(color)) => (*color, Color::DarkGray),
                    _ => (Color::Reset, Color::Reset),
                };
                let modifier_cursor = if x == cursor.x && y == cursor.y {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                };
                let modifier_uncommitted = match region_type {
                    Some(RegionType::Uncommitted) => Modifier::REVERSED,
                    _ => Modifier::empty(),
                };
                let style = Style::default()
                    .fg(fg)
                    .bg(bg)
                    .add_modifier(modifier_cursor | modifier_uncommitted);

                let buf_x = top_left_x + x as u16 + 1;
                let buf_y = top_left_y + y as u16 + 1;
                let cell = buf.get_mut(buf_x, buf_y);
                cell.set_char(c);
                cell.set_style(style);
            }
        }

        if game.is_complete() {
            let style = Style::default().bg(Color::Green);
            let points = chain!(
                iproduct!(0..(game_width + 2), [0, game_height + 1]),
                iproduct!([0, game_width + 1], 0..(game_height + 2)),
            );
            for (x, y) in points {
                let buf_x = top_left_x + x as u16;
                let buf_y = top_left_y + y as u16;
                let cell = buf.get_mut(buf_x, buf_y);
                cell.set_char(' ');
                cell.set_style(style);
            }
        }
    }

    fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let GameWidget {
            game, uncommitted, ..
        } = self;

        let status_text = if uncommitted.size() == 0 {
            "".to_owned()
        } else {
            match game.check_region(uncommitted) {
                Ok(_) => {
                    let word = uncommitted.word(game.board());
                    format!("\"{word}\"")
                }
                Err(CheckRegionError::TooShort) => "word too short".to_owned(),
                Err(CheckRegionError::TooLong) => "word too long".to_owned(),
                Err(CheckRegionError::OutOfBounds) => "region out of bounds (wtf)".to_owned(),
                Err(CheckRegionError::Overlapping) => "region overlapping (wtf)".to_owned(),
                Err(CheckRegionError::NotContiguous) => "region must be contiguous".to_owned(),
                Err(CheckRegionError::NotInDictionary) => {
                    let word = uncommitted.word(game.board());
                    format!("unknown word \"{word}\"")
                }
            }
        };

        Paragraph::new(status_text)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::REVERSED))
            .render(area, buf);
    }
}

impl<'a, 'b, 'c> Widget for GameWidget<'a, 'b, 'c> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        self.render_board(chunks[0], buf);
        self.render_status(chunks[1], buf);
    }
}

struct App<'a> {
    game: Game<'a, Color>,
    colors: Vec<Color>,
    all_colors: Vec<Color>,
    cursor: Square,
    uncommitted: Region,
    running: bool,
}

impl<'a> App<'a> {
    fn new(game: Game<'a, Color>) -> Self {
        let all_colors = vec![
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
        ];

        Self {
            game,
            colors: all_colors.clone(),
            all_colors,
            cursor: (0, 0).into(),
            uncommitted: Region::new(),
            running: true,
        }
    }

    fn on_event(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.running = false,
                KeyCode::Char('w') | KeyCode::Up => self.cursor_up(),
                KeyCode::Char('s') | KeyCode::Down => self.cursor_down(),
                KeyCode::Char('a') | KeyCode::Left => self.cursor_left(),
                KeyCode::Char('d') | KeyCode::Right => self.cursor_right(),
                KeyCode::Char(' ') => self.select(),
                KeyCode::Enter => self.add(),
                KeyCode::Delete => self.remove(),
                KeyCode::Insert => self.remove_and_add(),
                _ => {}
            },
            _ => {}
        }
    }

    fn draw<B: Backend>(&self, f: &mut Frame<'_, B>) {
        let size = f.size();
        let game_widget = GameWidget {
            game: &self.game,
            cursor: self.cursor,
            uncommitted: &self.uncommitted,
        };
        f.render_widget(game_widget, size);
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn cursor_up(&mut self) {
        if self.cursor.y > 0 {
            self.cursor.y -= 1;
        }
    }

    fn cursor_down(&mut self) {
        let max_y = self.game.board().height() - 1;
        if self.cursor.y < max_y {
            self.cursor.y += 1;
        }
    }

    fn cursor_left(&mut self) {
        if self.cursor.x > 0 {
            self.cursor.x -= 1;
        }
    }

    fn cursor_right(&mut self) {
        let max_x = self.game.board().width() - 1;
        if self.cursor.x < max_x {
            self.cursor.x += 1;
        }
    }

    fn select(&mut self) {
        // try removing the square - if it wasn't in the region, we're trying to add it
        if !self.uncommitted.remove_square(self.cursor) {
            // only add it if it's not currently occupied
            if self.game.is_square_free(self.cursor) {
                self.uncommitted.add_square(self.cursor);
            }
        }
    }

    fn add(&mut self) {
        // if the region is ok to add, add it
        if let Ok(checked_region) = self.game.check_region(&self.uncommitted) {
            // grab the next possible color, and refresh the list if we've run out
            let color = self.colors.pop().unwrap();
            if self.colors.is_empty() {
                self.colors = self.all_colors.clone();
            }

            // actually add the region and reset the uncommitted region
            self.game.add_region(checked_region, color);
            self.uncommitted = Region::new();
        }
    }

    fn remove(&mut self) {
        // try removing the committed region under the cursor, but if there is none, reset the
        // uncommitted region
        if self.game.remove_region(self.cursor).is_none() {
            self.uncommitted = Region::new();
        }
    }

    fn remove_and_add(&mut self) {
        // if the cursor is in a committed region, remove it and add every square from it to our
        // uncommitted region
        if let Some((region, _)) = self.game.remove_region(self.cursor) {
            for square in region.squares() {
                self.uncommitted.add_square(square);
            }
        }
    }
}

pub fn run(game: Game<Color>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(game);
    while app.is_running() {
        terminal.draw(|f| app.draw(f))?;

        if event::poll(Duration::from_millis(100))? {
            app.on_event(event::read()?);
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
