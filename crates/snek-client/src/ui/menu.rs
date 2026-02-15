use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Widget},
};

pub struct PauseMenu {
    pub selected: u8,
}

impl Widget for PauseMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_width: u16 = 22;
        let menu_height: u16 = 7;
        let menu_area = Rect {
            x: area.x + (area.width.saturating_sub(menu_width)) / 2,
            y: area.y + (area.height.saturating_sub(menu_height)) / 2,
            width: menu_width.min(area.width),
            height: menu_height.min(area.height),
        };

        Clear.render(menu_area, buf);

        let title = Line::from(" Paused ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        let options = ["Resume", "Restart", "Quit"];
        for (i, option) in options.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            let y = inner.y + i as u16;
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

pub struct GameOverMenu {
    pub selected: u8,
    pub score: u16,
}

impl Widget for GameOverMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let menu_width: u16 = 24;
        let menu_height: u16 = 7;
        let menu_area = Rect {
            x: area.x + (area.width.saturating_sub(menu_width)) / 2,
            y: area.y + (area.height.saturating_sub(menu_height)) / 2,
            width: menu_width.min(area.width),
            height: menu_height.min(area.height),
        };

        Clear.render(menu_area, buf);

        let title = Line::from(" Game Over ".bold().red());
        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        // Score line
        let score_text = format!("  Score: {}", self.score);
        for (j, ch) in score_text.chars().enumerate() {
            if j as u16 >= inner.width {
                break;
            }
            let cell = &mut buf[(inner.x + j as u16, inner.y)];
            cell.set_char(ch);
            cell.set_style(Style::default().fg(ratatui::style::Color::Green));
        }

        let options = ["Restart", "Quit"];
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
