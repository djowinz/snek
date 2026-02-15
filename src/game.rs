use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use rand::RngExt;
use ratatui::DefaultTerminal;

use crate::snake::Snake;
use crate::types::{KeyDirection, SpacePoint};
use crate::ui::{GameOverMenu, PauseMenu};

pub struct GameState {
    pub snake: Snake,
    pub food: SpacePoint,
    pub width: u16,
    pub height: u16,
    pub running: bool,
    pub winner: bool,
    pub speed: u64,
    pub selected_option: u8,
}

impl GameState {
    pub fn new(width: u16, height: u16) -> Self {
        let snake = Snake::new(width, height);
        let mut state = Self {
            snake,
            food: SpacePoint { x: 0, y: 0 },
            width,
            height,
            running: true,
            winner: false,
            speed: 100,
            selected_option: 0,
        };
        state.generate_food();
        state
    }

    pub fn generate_food(&mut self) {
        let mut rng = rand::rng();
        loop {
            let ran_x = rng.random_range(0..self.width);
            let ran_y = rng.random_range(0..self.height);
            let new_food = SpacePoint { x: ran_x, y: ran_y };

            if !self.snake.body.contains(&new_food) {
                self.food = new_food;
                break;
            }
        }
    }

    pub fn reset(&mut self) {
        let new = GameState::new(self.width, self.height);
        self.snake = new.snake;
        self.food = new.food;
        self.running = new.running;
        self.winner = new.winner;
        self.speed = new.speed;
        self.selected_option = 0;
    }

    pub fn update(&mut self) {
        let new_head = self.snake.advance(self.width, self.height);

        if !self.snake.alive {
            self.running = false;
            return;
        }

        if new_head != self.food {
            self.snake.body.pop_back();
        } else {
            self.snake.score += 1;
            if self.snake.score == (self.width * self.height) - 3 {
                self.running = false;
                self.winner = true;
            } else {
                let new_speed = self.speed - 5;
                if new_speed > 10 {
                    self.speed = new_speed;
                } else {
                    self.speed = 10;
                }
                self.generate_food();
            }
        }
    }
}

pub fn run(mut terminal: DefaultTerminal, mut game_state: GameState) -> Result<()> {
    loop {
        let paused = !game_state.running && game_state.snake.alive;
        let dead = !game_state.snake.alive;

        if crossterm::event::poll(std::time::Duration::from_millis(game_state.speed))?
            && let Event::Key(key) = event::read()?
        {
            if dead {
                match key.code {
                    KeyCode::Up | KeyCode::Char('w') => {
                        game_state.selected_option = if game_state.selected_option == 0 {
                            1
                        } else {
                            0
                        };
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        game_state.selected_option = (game_state.selected_option + 1) % 2;
                    }
                    KeyCode::Enter => match game_state.selected_option {
                        0 => game_state.reset(),
                        1 => break Result::Ok(()),
                        _ => {}
                    },
                    KeyCode::Esc => break Result::Ok(()),
                    _ => {}
                }
            } else if paused {
                match key.code {
                    KeyCode::Up | KeyCode::Char('w') => {
                        game_state.selected_option = if game_state.selected_option == 0 {
                            2
                        } else {
                            game_state.selected_option - 1
                        };
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        game_state.selected_option = (game_state.selected_option + 1) % 3;
                    }
                    KeyCode::Char('q') => {
                        game_state.running = true;
                    }
                    KeyCode::Enter => match game_state.selected_option {
                        0 => game_state.running = true,
                        1 => game_state.reset(),
                        2 => break Result::Ok(()),
                        _ => {}
                    },
                    KeyCode::Esc => break Result::Ok(()),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Up | KeyCode::Char('w') => {
                        if !matches!(game_state.snake.direction, KeyDirection::Down) {
                            game_state.snake.direction = KeyDirection::Up;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        if !matches!(game_state.snake.direction, KeyDirection::Up) {
                            game_state.snake.direction = KeyDirection::Down;
                        }
                    }
                    KeyCode::Left | KeyCode::Char('a') => {
                        if !matches!(game_state.snake.direction, KeyDirection::Right) {
                            game_state.snake.direction = KeyDirection::Left;
                        }
                    }
                    KeyCode::Right | KeyCode::Char('d') => {
                        if !matches!(game_state.snake.direction, KeyDirection::Left) {
                            game_state.snake.direction = KeyDirection::Right;
                        }
                    }
                    KeyCode::Esc => break Result::Ok(()),
                    KeyCode::Char('q') => {
                        game_state.running = false;
                        game_state.selected_option = 0;
                    }
                    _ => {}
                }
            }
        }

        if game_state.snake.alive && game_state.running {
            game_state.update();
        }

        if !game_state.snake.alive {
            game_state.selected_option = game_state.selected_option.min(1);
        }

        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(&game_state, area);
            if dead {
                frame.render_widget(
                    GameOverMenu {
                        selected: game_state.selected_option,
                        score: game_state.snake.score,
                    },
                    area,
                );
            } else if paused {
                frame.render_widget(
                    PauseMenu {
                        selected: game_state.selected_option,
                    },
                    area,
                );
            }
        })?;
    }
}
