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

pub struct NameInputScreen {
    pub name: String,
    pub cursor_pos: usize,
}

impl NameInputScreen {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            cursor_pos: 0,
        }
    }
}

impl Widget for &NameInputScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_width: u16 = 34;
        let menu_height: u16 = 7;
        let menu_area = Rect {
            x: area.x + (area.width.saturating_sub(menu_width)) / 2,
            y: area.y + (area.height.saturating_sub(menu_height)) / 2,
            width: menu_width.min(area.width),
            height: menu_height.min(area.height),
        };

        Clear.render(menu_area, buf);

        let title = Line::from(" Snek Game ".bold());
        let instructions = Line::from(" Enter to confirm ".blue().bold());
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::ROUNDED);

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        // Label
        let label = "Enter your name:";
        for (j, ch) in label.chars().enumerate() {
            if j as u16 >= inner.width {
                break;
            }
            let cell = &mut buf[(inner.x + j as u16, inner.y)];
            cell.set_char(ch);
            cell.set_style(Style::default().bold());
        }

        // Input field with underscore cursor
        let display: String = if self.name.is_empty() {
            "_".repeat(16)
        } else {
            let remaining = 16usize.saturating_sub(self.name.len());
            format!("{}{}", self.name, "_".repeat(remaining))
        };

        let field_y = inner.y + 2;
        let field_x = inner.x + (inner.width.saturating_sub(16)) / 2;
        for (j, ch) in display.chars().enumerate() {
            if j as u16 >= inner.width {
                break;
            }
            let style = if j == self.cursor_pos {
                Style::default().fg(ratatui::style::Color::Yellow).bold()
            } else if j < self.name.len() {
                Style::default().fg(ratatui::style::Color::Green)
            } else {
                Style::default().fg(ratatui::style::Color::DarkGray)
            };
            let cell = &mut buf[(field_x + j as u16, field_y)];
            cell.set_char(ch);
            cell.set_style(style);
        }
    }
}

pub enum NameInputResult {
    Name(String),
    Quit,
}

pub fn run_name_input(terminal: &mut DefaultTerminal) -> Result<NameInputResult> {
    let mut screen = NameInputScreen::new();

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&screen, frame.area());
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => {
                    if !screen.name.is_empty() {
                        return Ok(NameInputResult::Name(screen.name));
                    }
                }
                KeyCode::Esc => return Ok(NameInputResult::Quit),
                KeyCode::Backspace => {
                    if !screen.name.is_empty() {
                        screen.name.pop();
                        screen.cursor_pos = screen.name.len();
                    }
                }
                KeyCode::Char(c) => {
                    if screen.name.len() < 16 && c.is_alphanumeric() {
                        screen.name.push(c);
                        screen.cursor_pos = screen.name.len();
                    }
                }
                _ => {}
            }
        }
    }
}
