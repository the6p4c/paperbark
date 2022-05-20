use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::collections::HashMap;
use std::io;
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
    temp_region: &'c Region,
}

impl<'a, 'b, 'c> Widget for GameWidget<'a, 'b, 'c> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let GameWidget {
            game,
            cursor,
            temp_region,
        } = self;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        let (chunk_board, chunk_status) = (chunks[0], chunks[1]);

        let game_width = game.board().width();
        let game_height = game.board().height();

        assert!(chunk_board.width >= game_width as u16);
        assert!(chunk_board.height >= game_height as u16);

        let top_left_x = chunk_board.x + (chunk_board.width - game_width as u16) / 2;
        let top_left_y = chunk_board.y + (chunk_board.height - game_height as u16) / 2;

        let square_to_region_type = game
            .regions()
            .flat_map(|(region, data)| {
                region
                    .squares()
                    .map(move |square| (square, RegionType::Committed(*data)))
            })
            .chain(
                temp_region
                    .squares()
                    .map(|square| (square, RegionType::Uncommitted)),
            )
            .collect::<HashMap<_, _>>();

        for y in 0..game_height {
            for x in 0..game_width {
                let square = (x, y).into();
                let c = game.board().get(square);
                let region_type = square_to_region_type.get(&square);

                let fg = match region_type {
                    Some(RegionType::Committed(color)) => *color,
                    _ => Color::Reset,
                };
                let modifier_cursor = if x == cursor.x && y == cursor.y {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                };
                let modifier_temp_region = match region_type {
                    Some(RegionType::Uncommitted) => Modifier::REVERSED,
                    _ => Modifier::empty(),
                };
                let style = Style::default()
                    .fg(fg)
                    .add_modifier(modifier_cursor | modifier_temp_region);

                let buf_x = top_left_x + x as u16;
                let buf_y = top_left_y + y as u16;
                let cell = buf.get_mut(buf_x, buf_y);
                cell.set_char(c);
                cell.set_style(style);
            }
        }

        let status_text = if temp_region.size() == 0 {
            "".to_owned()
        } else {
            match game.check_region(&temp_region) {
                Ok(_) => {
                    let word = temp_region.word(game.board());
                    format!("\"{word}\"")
                }
                Err(CheckRegionError::TooShort) => "word too short".to_owned(),
                Err(CheckRegionError::TooLong) => "word too long".to_owned(),
                Err(CheckRegionError::OutOfBounds) => "region out of bounds (wtf)".to_owned(),
                Err(CheckRegionError::Overlapping) => "region overlapping (wtf)".to_owned(),
                Err(CheckRegionError::NotContiguous) => "region must be contiguous".to_owned(),
                Err(CheckRegionError::NotInDictionary) => {
                    let word = temp_region.word(game.board());
                    format!("unknown word \"{word}\"")
                }
            }
        };

        Paragraph::new(status_text)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::REVERSED))
            .render(chunk_status, buf);
    }
}

struct App<'a> {
    game: Game<'a, Color>,
    cursor: Square,
    temp_region: Region,
    running: bool,
}

impl<'a> App<'a> {
    fn new(game: Game<'a, Color>) -> Self {
        Self {
            game,
            cursor: (0, 0).into(),
            temp_region: Region::new(),
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
            temp_region: &self.temp_region,
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
        if !self.temp_region.remove_square(self.cursor) {
            // only add it if it's not currently occupied
            if self.game.is_square_free(self.cursor) {
                self.temp_region.add_square(self.cursor);
            }
        }
    }

    fn add(&mut self) {
        if let Ok(checked_region) = self.game.check_region(&self.temp_region) {
            self.game.add_region(checked_region, Color::Red);
            self.temp_region = Region::new();
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

        if event::poll(std::time::Duration::from_secs(0))? {
            app.on_event(event::read()?);
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
