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

pub struct SearchingScreen {
    pub dots: u8,
}

impl Widget for &SearchingScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_width: u16 = 36;
        let menu_height: u16 = 5;
        let menu_area = Rect {
            x: area.x + (area.width.saturating_sub(menu_width)) / 2,
            y: area.y + (area.height.saturating_sub(menu_height)) / 2,
            width: menu_width.min(area.width),
            height: menu_height.min(area.height),
        };

        Clear.render(menu_area, buf);

        let title = Line::from(" Multiplayer ".bold());
        let instructions = Line::from(" Esc to cancel ".blue().bold());
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::ROUNDED);

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        let dot_str = ".".repeat(self.dots as usize);
        let text = format!("Searching for opponent{dot_str}");
        for (j, ch) in text.chars().enumerate() {
            if j as u16 >= inner.width {
                break;
            }
            let cell = &mut buf[(inner.x + j as u16, inner.y + inner.height / 2)];
            cell.set_char(ch);
            cell.set_style(Style::default().fg(ratatui::style::Color::Yellow));
        }
    }
}

pub enum SearchResult {
    Cancelled,
    MessageReceived(String),
}

/// Poll for either user cancellation (Esc) or a network message.
/// Returns when either happens. Animates dots while waiting.
pub fn run_searching(
    terminal: &mut DefaultTerminal,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<String>,
) -> Result<SearchResult> {
    let mut screen = SearchingScreen { dots: 0 };
    let mut tick_count: u32 = 0;

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&screen, frame.area());
        })?;

        // Poll input with short timeout for animation
        if crossterm::event::poll(std::time::Duration::from_millis(300))?
            && let Event::Key(key) = event::read()?
            && key.code == KeyCode::Esc
        {
            return Ok(SearchResult::Cancelled);
        }

        // Check for network messages (non-blocking)
        if let Ok(msg) = rx.try_recv() {
            return Ok(SearchResult::MessageReceived(msg));
        }

        tick_count += 1;
        screen.dots = (tick_count % 4) as u8;
    }
}
