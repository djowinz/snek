use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Widget},
};

pub struct TerminalTooSmallOverlay {
    pub min_width: u16,
    pub min_height: u16,
}

impl Widget for TerminalTooSmallOverlay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_width: u16 = 36;
        let menu_height: u16 = 7;
        let menu_area = Rect {
            x: area.x + (area.width.saturating_sub(menu_width)) / 2,
            y: area.y + (area.height.saturating_sub(menu_height)) / 2,
            width: menu_width.min(area.width),
            height: menu_height.min(area.height),
        };

        Clear.render(menu_area, buf);

        let title = Line::from(" Warning ".bold().red());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        let line1 = "Terminal too small!";
        for (j, ch) in line1.chars().enumerate() {
            if j as u16 >= inner.width {
                break;
            }
            let cell = &mut buf[(inner.x + j as u16, inner.y)];
            cell.set_char(ch);
            cell.set_style(Style::default().bold().fg(ratatui::style::Color::Red));
        }

        let line2 = format!("Resize to at least {}x{}", self.min_width, self.min_height);
        let y2 = inner.y + 2;
        if y2 < inner.y + inner.height {
            for (j, ch) in line2.chars().enumerate() {
                if j as u16 >= inner.width {
                    break;
                }
                let cell = &mut buf[(inner.x + j as u16, y2)];
                cell.set_char(ch);
                cell.set_style(Style::default().fg(ratatui::style::Color::Yellow));
            }
        }
    }
}
