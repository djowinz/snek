use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use rand::RngExt;
use ratatui::DefaultTerminal;

use crate::network::NetworkConnection;
use crate::snake::Snake;
use crate::ui::board_mp::MultiplayerBoard;
use crate::ui::terminal_warning::TerminalTooSmallOverlay;
use snek_shared::*;

#[allow(dead_code)]
pub struct HostGameResult {
    pub result: MatchResult,
    pub reason: GameOverReason,
    pub final_snakes: Vec<SnakeState>,
    pub disconnected: bool,
}

pub fn run_host(
    terminal: &mut DefaultTerminal,
    net: &mut NetworkConnection,
    board_width: u16,
    board_height: u16,
    you: &PlayerInfo,
    opponent: &PlayerInfo,
) -> Result<HostGameResult> {
    // Send countdown
    for i in (1..=3).rev() {
        net.send_peer(&PeerMessage::Countdown { remaining: i })?;

        // Render countdown locally
        terminal.draw(|frame| {
            let screen = crate::screens::countdown::CountdownScreen {
                remaining: i,
                you,
                opponent,
            };
            frame.render_widget(&screen, frame.area());
        })?;

        // Wait 1 second, draining input
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            let remaining = deadline - std::time::Instant::now();
            if crossterm::event::poll(remaining)? {
                let _ = event::read()?; // discard input during countdown
            }
        }
    }

    // Initialize snakes
    let mut host_snake = Snake::new_at(board_width / 4, board_height / 2, KeyDirection::Right, 3);
    let mut guest_snake =
        Snake::new_at(3 * board_width / 4, board_height / 2, KeyDirection::Left, 3);

    // Generate initial food
    let mut food = generate_food_mp(board_width, board_height, &host_snake, &guest_snake);
    let mut tick: u64 = 0;
    let mut food_placed_tick: u64 = 0;
    const FOOD_TIMEOUT_TICKS: u64 = 50; // 5 seconds at 100ms/tick

    // Send initial game tick
    let snakes_state = build_snakes_state(you, opponent, &host_snake, &guest_snake);
    net.send_peer(&PeerMessage::GameTick {
        snakes: snakes_state.clone(),
        food,
        tick,
    })?;

    let tick_duration = std::time::Duration::from_millis(100);
    let mut terminal_too_small = {
        let dim = terminal.size()?;
        dim.width < MP_MIN_TERM_WIDTH || dim.height < MP_MIN_TERM_HEIGHT
    };

    loop {
        // Render
        let snakes_state = build_snakes_state(you, opponent, &host_snake, &guest_snake);
        let too_small = terminal_too_small;
        terminal.draw(|frame| {
            let board = MultiplayerBoard {
                snakes: &snakes_state,
                food,
                board_width,
                board_height,
            };
            frame.render_widget(&board, frame.area());
            if too_small {
                frame.render_widget(
                    TerminalTooSmallOverlay {
                        min_width: MP_MIN_TERM_WIDTH,
                        min_height: MP_MIN_TERM_HEIGHT,
                    },
                    frame.area(),
                );
            }
        })?;

        // Poll input
        if crossterm::event::poll(tick_duration)? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Up | KeyCode::Char('w') => {
                        if !matches!(host_snake.direction, KeyDirection::Down) {
                            host_snake.direction = KeyDirection::Up;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('s') => {
                        if !matches!(host_snake.direction, KeyDirection::Up) {
                            host_snake.direction = KeyDirection::Down;
                        }
                    }
                    KeyCode::Left | KeyCode::Char('a') => {
                        if !matches!(host_snake.direction, KeyDirection::Right) {
                            host_snake.direction = KeyDirection::Left;
                        }
                    }
                    KeyCode::Right | KeyCode::Char('d') => {
                        if !matches!(host_snake.direction, KeyDirection::Left) {
                            host_snake.direction = KeyDirection::Right;
                        }
                    }
                    KeyCode::Esc => {
                        // Host disconnecting = opponent wins
                        let final_snakes =
                            build_snakes_state(you, opponent, &host_snake, &guest_snake);
                        net.send_peer(&PeerMessage::GameOver {
                            result: MatchResult::Win,
                            reason: GameOverReason::OpponentDisconnected,
                            final_snakes: final_snakes.clone(),
                        })?;
                        return Ok(HostGameResult {
                            result: MatchResult::Loss,
                            reason: GameOverReason::OpponentDisconnected,
                            final_snakes,
                            disconnected: true,
                        });
                    }
                    _ => {}
                },
                Event::Resize(w, h) => {
                    terminal_too_small = w < MP_MIN_TERM_WIDTH || h < MP_MIN_TERM_HEIGHT;
                }
                _ => {}
            }
        }

        // Drain network messages from guest
        while let Ok(msg) = net.rx.try_recv() {
            if let Ok(relay_msg) = serde_json::from_str::<ServerRelayMessage>(&msg) {
                match relay_msg {
                    ServerRelayMessage::PeerEnvelope { payload } => {
                        if let Ok(peer_msg) = serde_json::from_str::<PeerMessage>(&payload) {
                            match peer_msg {
                                PeerMessage::ChangeDirection { direction } => {
                                    let opposite = matches!(
                                        (&guest_snake.direction, &direction),
                                        (KeyDirection::Up, KeyDirection::Down)
                                            | (KeyDirection::Down, KeyDirection::Up)
                                            | (KeyDirection::Left, KeyDirection::Right)
                                            | (KeyDirection::Right, KeyDirection::Left)
                                    );
                                    if !opposite {
                                        guest_snake.direction = direction;
                                    }
                                }
                                PeerMessage::LeaveGame => {
                                    let final_snakes = build_snakes_state(
                                        you,
                                        opponent,
                                        &host_snake,
                                        &guest_snake,
                                    );
                                    return Ok(HostGameResult {
                                        result: MatchResult::Win,
                                        reason: GameOverReason::OpponentDisconnected,
                                        final_snakes,
                                        disconnected: false,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                    ServerRelayMessage::OpponentDisconnected => {
                        let final_snakes =
                            build_snakes_state(you, opponent, &host_snake, &guest_snake);
                        return Ok(HostGameResult {
                            result: MatchResult::Win,
                            reason: GameOverReason::OpponentDisconnected,
                            final_snakes,
                            disconnected: true,
                        });
                    }
                    _ => {}
                }
            }
        }

        // Advance both snakes
        let host_head = host_snake.advance(board_width, board_height);
        let guest_head = guest_snake.advance(board_width, board_height);

        // Check collisions
        let mut host_died = !host_snake.alive;
        let mut guest_died = !guest_snake.alive;
        let mut host_reason = GameOverReason::SelfCollision;
        let mut guest_reason = GameOverReason::SelfCollision;

        // Head-on collision
        if host_head == guest_head {
            if host_snake.body.len() > guest_snake.body.len() {
                guest_died = true;
                guest_reason = GameOverReason::HeadOnCollisionLoss;
            } else if guest_snake.body.len() > host_snake.body.len() {
                host_died = true;
                host_reason = GameOverReason::HeadOnCollisionLoss;
            } else {
                host_died = true;
                guest_died = true;
                host_reason = GameOverReason::HeadOnCollisionLoss;
                guest_reason = GameOverReason::HeadOnCollisionLoss;
            }
        }

        // Body collision: host head in guest body (skip head since head-on handled above)
        if !host_died && guest_snake.body.iter().skip(1).any(|p| *p == host_head) {
            host_died = true;
            host_reason = GameOverReason::BodyCollision;
        }
        // Body collision: guest head in host body
        if !guest_died && host_snake.body.iter().skip(1).any(|p| *p == guest_head) {
            guest_died = true;
            guest_reason = GameOverReason::BodyCollision;
        }

        if host_died {
            host_snake.alive = false;
        }
        if guest_died {
            guest_snake.alive = false;
        }

        // Check food consumption
        let mut ate_food = false;
        if host_head == food && host_snake.alive {
            host_snake.score += 1;
            ate_food = true;
        } else {
            host_snake.body.pop_back();
        }
        if guest_head == food && guest_snake.alive {
            guest_snake.score += 1;
            ate_food = true;
        } else {
            guest_snake.body.pop_back();
        }

        if ate_food {
            food = generate_food_mp(board_width, board_height, &host_snake, &guest_snake);
            food_placed_tick = tick;
        }

        tick += 1;

        // Relocate food if not eaten within 3 seconds
        if tick - food_placed_tick >= FOOD_TIMEOUT_TICKS {
            food = generate_food_mp(board_width, board_height, &host_snake, &guest_snake);
            food_placed_tick = tick;
        }

        // Check game over
        if host_died || guest_died {
            let final_snakes = build_snakes_state(you, opponent, &host_snake, &guest_snake);

            let (host_result, guest_result) = if host_died && guest_died {
                (MatchResult::Draw, MatchResult::Draw)
            } else if host_died {
                (MatchResult::Loss, MatchResult::Win)
            } else {
                (MatchResult::Win, MatchResult::Loss)
            };

            // Send game over to guest (from guest's perspective)
            net.send_peer(&PeerMessage::GameOver {
                result: guest_result,
                reason: if guest_died {
                    guest_reason
                } else {
                    GameOverReason::OpponentDied
                },
                final_snakes: final_snakes.clone(),
            })?;

            return Ok(HostGameResult {
                result: host_result,
                reason: if host_died {
                    host_reason
                } else {
                    GameOverReason::OpponentDied
                },
                final_snakes,
                disconnected: false,
            });
        }

        // Send tick to guest
        let snakes_state = build_snakes_state(you, opponent, &host_snake, &guest_snake);
        net.send_peer(&PeerMessage::GameTick {
            snakes: snakes_state,
            food,
            tick,
        })?;
    }
}

fn build_snakes_state(
    you: &PlayerInfo,
    opponent: &PlayerInfo,
    host_snake: &Snake,
    guest_snake: &Snake,
) -> Vec<SnakeState> {
    vec![
        SnakeState {
            player: you.clone(),
            body: host_snake.body.iter().copied().collect(),
            direction: host_snake.direction,
            alive: host_snake.alive,
            score: host_snake.score,
        },
        SnakeState {
            player: opponent.clone(),
            body: guest_snake.body.iter().copied().collect(),
            direction: guest_snake.direction,
            alive: guest_snake.alive,
            score: guest_snake.score,
        },
    ]
}

fn generate_food_mp(width: u16, height: u16, snake1: &Snake, snake2: &Snake) -> SpacePoint {
    let mut rng = rand::rng();
    loop {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        let pt = SpacePoint { x, y };
        if !snake1.body.contains(&pt) && !snake2.body.contains(&pt) {
            return pt;
        }
    }
}
