use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Widget},
};

use snek_shared::PlayerInfo;

pub struct CountdownScreen<'a> {
    pub remaining: u8,
    pub you: &'a PlayerInfo,
    pub opponent: &'a PlayerInfo,
}

impl Widget for &CountdownScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_width: u16 = 36;
        let menu_height: u16 = 9;
        let menu_area = Rect {
            x: area.x + (area.width.saturating_sub(menu_width)) / 2,
            y: area.y + (area.height.saturating_sub(menu_height)) / 2,
            width: menu_width.min(area.width),
            height: menu_height.min(area.height),
        };

        Clear.render(menu_area, buf);

        let title = Line::from(" Get Ready! ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::ROUNDED);

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        // Player names
        let vs_text = format!("{} vs {}", self.you.name, self.opponent.name);
        let vs_x = inner.x + inner.width.saturating_sub(vs_text.len() as u16) / 2;
        for (j, ch) in vs_text.chars().enumerate() {
            if j as u16 + vs_x >= inner.x + inner.width {
                break;
            }
            let cell = &mut buf[(vs_x + j as u16, inner.y + 1)];
            cell.set_char(ch);
            cell.set_style(Style::default().bold());
        }

        // Countdown number
        let count_text = if self.remaining == 0 {
            "GO!".to_string()
        } else {
            format!("{}", self.remaining)
        };
        let count_x = inner.x + inner.width.saturating_sub(count_text.len() as u16) / 2;
        let count_y = inner.y + 3;
        for (j, ch) in count_text.chars().enumerate() {
            if count_x + j as u16 >= inner.x + inner.width {
                break;
            }
            let cell = &mut buf[(count_x + j as u16, count_y)];
            cell.set_char(ch);
            cell.set_style(Style::default().fg(ratatui::style::Color::Yellow).bold());
        }
    }
}
