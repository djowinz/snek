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

pub struct ModeSelectScreen {
    pub player_name: String,
    pub selected: u8,
}

impl Widget for &ModeSelectScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_width: u16 = 30;
        let menu_height: u16 = 9;
        let menu_area = Rect {
            x: area.x + (area.width.saturating_sub(menu_width)) / 2,
            y: area.y + (area.height.saturating_sub(menu_height)) / 2,
            width: menu_width.min(area.width),
            height: menu_height.min(area.height),
        };

        Clear.render(menu_area, buf);

        let title = Line::from(" Snek Game ".bold());
        let player_line = Line::from(format!(" {} ", self.player_name).green().bold());
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(player_line.centered())
            .border_set(border::ROUNDED);

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        // Header
        let label = "Select Mode:";
        for (j, ch) in label.chars().enumerate() {
            if j as u16 >= inner.width {
                break;
            }
            let cell = &mut buf[(inner.x + j as u16, inner.y)];
            cell.set_char(ch);
            cell.set_style(Style::default().bold());
        }

        let options = ["Single Player", "Multiplayer", "Quit"];
        for (i, option) in options.iter().enumerate() {
            let y = inner.y + 2 + i as u16;
            if y >= inner.y + inner.height {
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
                let cell = &mut buf[(inner.x + j as u16, y)];
                cell.set_char(ch);
                cell.set_style(style);
            }
        }
    }
}

#[derive(PartialEq)]
pub enum ModeSelection {
    SinglePlayer,
    Multiplayer,
    Quit,
}

pub fn run_mode_select(terminal: &mut DefaultTerminal, player_name: &str) -> Result<ModeSelection> {
    let mut screen = ModeSelectScreen {
        player_name: player_name.to_string(),
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
                        0 => ModeSelection::SinglePlayer,
                        1 => ModeSelection::Multiplayer,
                        _ => ModeSelection::Quit,
                    });
                }
                KeyCode::Esc => return Ok(ModeSelection::Quit),
                _ => {}
            }
        }
    }
}
