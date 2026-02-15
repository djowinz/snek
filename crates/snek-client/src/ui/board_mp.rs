use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Widget},
};

use snek_shared::{SnakeColor, SnakeState, SpacePoint};

fn snake_color_to_ratatui(color: SnakeColor) -> ratatui::style::Color {
    match color {
        SnakeColor::Blue => ratatui::style::Color::Blue,
        SnakeColor::Magenta => ratatui::style::Color::Magenta,
        SnakeColor::Cyan => ratatui::style::Color::Cyan,
        SnakeColor::Yellow => ratatui::style::Color::Yellow,
        SnakeColor::Green => ratatui::style::Color::Green,
        SnakeColor::Red => ratatui::style::Color::Red,
    }
}

pub struct MultiplayerBoard<'a> {
    pub snakes: &'a [SnakeState],
    pub food: SpacePoint,
    pub board_width: u16,
    pub board_height: u16,
}

impl Widget for &MultiplayerBoard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Build title with colored player names and scores
        let title = Line::from(" Snek Multiplayer ".bold());

        let score_spans: Vec<Span> = self
            .snakes
            .iter()
            .enumerate()
            .flat_map(|(i, snake)| {
                let color = snake_color_to_ratatui(snake.player.color);
                let mut spans = Vec::new();
                if i > 0 {
                    spans.push(Span::raw(" | "));
                }
                spans.push(Span::styled(
                    format!("{}: {}", snake.player.name, snake.score),
                    Style::default().fg(color).bold(),
                ));
                spans
            })
            .collect();

        let score_line = Line::from(score_spans);
        let instructions = Line::from(vec![
            " Arrow keys / WASD to move ".blue().bold(),
            " Esc to quit ".blue().bold(),
        ]);

        let block = Block::bordered()
            .title(title.left_aligned())
            .title(score_line.centered())
            .title_bottom(instructions.centered())
            .border_set(border::ROUNDED);

        let playarea: Rect = block.inner(area);

        // Render the food
        if self.food.x < playarea.width && self.food.y < playarea.height {
            let food_cell = &mut buf[(playarea.x + self.food.x, playarea.y + self.food.y)];
            food_cell.set_char('#');
            food_cell.set_style(Style::default().fg(ratatui::style::Color::Green));
        }

        // Render all snakes
        for snake in self.snakes {
            let color = snake_color_to_ratatui(snake.player.color);
            let style = if snake.alive {
                Style::default().fg(color)
            } else {
                Style::default().fg(ratatui::style::Color::DarkGray)
            };
            for (idx, pt) in snake.body.iter().enumerate() {
                if pt.x < playarea.width && pt.y < playarea.height {
                    let cell = &mut buf[(playarea.x + pt.x, playarea.y + pt.y)];
                    // Head gets a different char
                    if idx == 0 {
                        cell.set_char('O');
                    } else {
                        cell.set_char('█');
                    }
                    cell.set_style(style);
                }
            }
        }

        block.render(area, buf);
    }
}
