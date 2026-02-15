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

fn lives_string(lives: u8) -> String {
    if lives == 0 {
        return String::new();
    }
    format!(" [{}]", "\u{2665}".repeat(lives as usize))
}

pub struct MultiplayerBoard<'a> {
    pub snakes: &'a [SnakeState],
    pub food: SpacePoint,
    pub bombs: &'a [SpacePoint],
    pub board_width: u16,
    pub board_height: u16,
    pub time_remaining: u16,
}

impl Widget for &MultiplayerBoard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Compute centered board rect (board_width+2 x board_height+2 for border)
        let total_w = self.board_width + 2;
        let total_h = self.board_height + 2;
        let board_area = Rect {
            x: area.x + area.width.saturating_sub(total_w) / 2,
            y: area.y + area.height.saturating_sub(total_h) / 2,
            width: total_w.min(area.width),
            height: total_h.min(area.height),
        };

        // Build title with colored player names, scores, and lives
        let title = Line::from(" Snek Game ".bold());

        // Timer display
        let minutes = self.time_remaining / 60;
        let seconds = self.time_remaining % 60;
        let timer_line = Line::from(format!(" {minutes}:{seconds:02} ").bold().yellow());

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
                    format!(
                        "{}: {}{}",
                        snake.player.name,
                        snake.score,
                        lives_string(snake.lives)
                    ),
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
            .title(timer_line.right_aligned())
            .title(score_line.centered())
            .title_bottom(instructions.centered())
            .border_set(border::ROUNDED);

        let playarea: Rect = block.inner(board_area);

        // Render the food
        if self.food.x < playarea.width && self.food.y < playarea.height {
            let food_cell = &mut buf[(playarea.x + self.food.x, playarea.y + self.food.y)];
            food_cell.set_char('#');
            food_cell.set_style(Style::default().fg(ratatui::style::Color::Green));
        }

        // Render bombs
        for bomb in self.bombs {
            if bomb.x < playarea.width && bomb.y < playarea.height {
                let cell = &mut buf[(playarea.x + bomb.x, playarea.y + bomb.y)];
                cell.set_char('B');
                cell.set_style(Style::default().fg(ratatui::style::Color::Green));
            }
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

        block.render(board_area, buf);
    }
}
