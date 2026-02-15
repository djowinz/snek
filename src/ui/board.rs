use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Widget},
};

use crate::game::GameState;

impl Widget for &GameState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Snek Game ".bold());
        let direction = Line::from(format!(" Direction: {} ", self.snake.direction));
        let game_speed = Line::from(format!(" Game Speed: {} ", self.speed));
        let score = Line::from(format!(" Score: {} ", self.snake.score).green().bold());
        let instructions = Line::from(vec![
            " Use arrow keys or WASD to move ".blue().bold(),
            "Eat food to grow ".bold(),
            "Press 'q' to quit ".blue().bold(),
        ]);

        let block = Block::bordered()
            .title(title.left_aligned())
            .title(direction.centered())
            .title(game_speed.centered())
            .title(score.right_aligned())
            .title_bottom(instructions.centered())
            .border_set(border::ROUNDED);

        let playarea: Rect = block.inner(area);

        // Render the food
        let food_cell = &mut buf[(playarea.x + self.food.x, playarea.y + self.food.y)];
        food_cell.set_char('#');
        food_cell.set_style(Style::default().fg(ratatui::style::Color::Green));

        // Render the body of the snek using block elements
        for vec in self.snake.body.iter() {
            let body_cell = &mut buf[(playarea.x + vec.x, playarea.y + vec.y)];
            body_cell.set_char('█');
            body_cell.set_style(Style::default().fg(ratatui::style::Color::Blue));
        }

        block.render(area, buf);
    }
}
