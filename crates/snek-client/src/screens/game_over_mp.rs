use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Widget},
};
use snek_shared::{MatchResult, SnakeState};

pub struct GameOverMpScreen<'a> {
    pub result: MatchResult,
    pub snakes: &'a [SnakeState],
    pub selected: u8,
}

impl Widget for &GameOverMpScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_width: u16 = 34;
        let menu_height: u16 = 12;
        let menu_area = Rect {
            x: area.x + (area.width.saturating_sub(menu_width)) / 2,
            y: area.y + (area.height.saturating_sub(menu_height)) / 2,
            width: menu_width.min(area.width),
            height: menu_height.min(area.height),
        };

        Clear.render(menu_area, buf);

        let title_text = match self.result {
            MatchResult::Win => " You Win! ",
            MatchResult::Loss => " You Lost ",
            MatchResult::Draw => " Draw! ",
        };
        let title_color = match self.result {
            MatchResult::Win => ratatui::style::Color::Green,
            MatchResult::Loss => ratatui::style::Color::Red,
            MatchResult::Draw => ratatui::style::Color::Yellow,
        };
        let title = Line::from(title_text.bold().fg(title_color));
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        // Scores
        let mut y = inner.y;
        for snake in self.snakes {
            let text = format!("  {}: {}", snake.player.name, snake.score);
            for (j, ch) in text.chars().enumerate() {
                if j as u16 >= inner.width {
                    break;
                }
                let cell = &mut buf[(inner.x + j as u16, y)];
                cell.set_char(ch);
                cell.set_style(Style::default().fg(ratatui::style::Color::Green));
            }
            y += 1;
        }

        y += 1; // spacer

        let options = ["Search Again", "Quit to Menu", "Quit"];
        for (i, option) in options.iter().enumerate() {
            let row = y + i as u16;
            if row >= inner.y + inner.height {
                break;
            }
            let (prefix, style) = if i as u8 == self.selected {
                (
                    "> ",
                    Style::default().bold().fg(ratatui::style::Color::Yellow),
                )
            } else {
                ("  ", Style::default())
            };
            let text = format!("{prefix}{option}");
            for (j, ch) in text.chars().enumerate() {
                if j as u16 >= inner.width {
                    break;
                }
                let cell = &mut buf[(inner.x + j as u16, row)];
                cell.set_char(ch);
                cell.set_style(style);
            }
        }
    }
}

#[derive(PartialEq)]
pub enum GameOverMpChoice {
    SearchAgain,
    QuitToMenu,
    Quit,
}

pub fn run_game_over_mp(
    terminal: &mut DefaultTerminal,
    result: MatchResult,
    snakes: &[SnakeState],
) -> Result<GameOverMpChoice> {
    let mut screen = GameOverMpScreen {
        result,
        snakes,
        selected: 0,
    };

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&screen, frame.area());
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Up | KeyCode::Char('w') => {
                    screen.selected = if screen.selected == 0 {
                        2
                    } else {
                        screen.selected - 1
                    };
                }
                KeyCode::Down | KeyCode::Char('s') => {
                    screen.selected = (screen.selected + 1) % 3;
                }
                KeyCode::Enter => {
                    return Ok(match screen.selected {
                        0 => GameOverMpChoice::SearchAgain,
                        1 => GameOverMpChoice::QuitToMenu,
                        _ => GameOverMpChoice::Quit,
                    });
                }
                KeyCode::Esc => return Ok(GameOverMpChoice::QuitToMenu),
                _ => {}
            }
        }
    }
}
