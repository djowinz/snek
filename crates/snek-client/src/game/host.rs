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

    // Spawn positions (stored for respawning)
    let host_spawn = (board_width / 4, board_height / 2, KeyDirection::Right);
    let guest_spawn = (3 * board_width / 4, board_height / 2, KeyDirection::Left);

    // Initialize snakes
    let mut host_snake = Snake::new_at(host_spawn.0, host_spawn.1, host_spawn.2, 3);
    let mut guest_snake = Snake::new_at(guest_spawn.0, guest_spawn.1, guest_spawn.2, 3);

    // Generate initial food
    let mut bombs: Vec<SpacePoint> = Vec::new();
    let mut food = generate_food_mp(board_width, board_height, &host_snake, &guest_snake, &bombs);
    let mut tick: u64 = 0;
    let mut food_placed_tick: u64 = 0;
    let mut last_bomb_tick: u64 = 0;
    const FOOD_TIMEOUT_TICKS: u64 = 50; // 5 seconds at 100ms/tick

    // Compute initial time remaining
    let time_remaining = compute_time_remaining(tick);

    // Send initial game tick
    let snakes_state = build_snakes_state(you, opponent, &host_snake, &guest_snake);
    net.send_peer(&PeerMessage::GameTick {
        snakes: snakes_state.clone(),
        food,
        bombs: bombs.clone(),
        tick,
        time_remaining,
    })?;

    let tick_duration = std::time::Duration::from_millis(100);
    let mut terminal_too_small = {
        let dim = terminal.size()?;
        dim.width < MP_MIN_TERM_WIDTH || dim.height < MP_MIN_TERM_HEIGHT
    };

    loop {
        // Render
        let time_remaining = compute_time_remaining(tick);
        let snakes_state = build_snakes_state(you, opponent, &host_snake, &guest_snake);
        let too_small = terminal_too_small;
        let bombs_snapshot = bombs.clone();
        terminal.draw(|frame| {
            let board = MultiplayerBoard {
                snakes: &snakes_state,
                food,
                bombs: &bombs_snapshot,
                board_width,
                board_height,
                time_remaining,
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
            food = generate_food_mp(board_width, board_height, &host_snake, &guest_snake, &bombs);
            food_placed_tick = tick;
        }

        // Check bomb consumption (after food, so tail was already popped for non-food moves)
        if host_snake.alive
            && let Some(idx) = bombs.iter().position(|b| *b == host_head)
        {
            bombs.remove(idx);
            if host_snake.body.len() <= 1 {
                // Only head left — bomb kills the snake
                host_snake.alive = false;
                host_died = true;
                host_reason = GameOverReason::BombExplosion;
            } else {
                host_snake.body.pop_back();
            }
        }
        if guest_snake.alive
            && let Some(idx) = bombs.iter().position(|b| *b == guest_head)
        {
            bombs.remove(idx);
            if guest_snake.body.len() <= 1 {
                guest_snake.alive = false;
                guest_died = true;
                guest_reason = GameOverReason::BombExplosion;
            } else {
                guest_snake.body.pop_back();
            }
        }

        tick += 1;

        // Relocate food if not eaten within timeout
        if tick - food_placed_tick >= FOOD_TIMEOUT_TICKS {
            food = generate_food_mp(board_width, board_height, &host_snake, &guest_snake, &bombs);
            food_placed_tick = tick;
        }

        // Bomb spawning / moving
        if tick - last_bomb_tick >= BOMB_SPAWN_INTERVAL_TICKS {
            if bombs.len() < MAX_BOMBS {
                // Spawn a new bomb
                if let Some(pos) = generate_bomb_position(
                    board_width,
                    board_height,
                    &host_snake,
                    &guest_snake,
                    &bombs,
                    food,
                ) {
                    bombs.push(pos);
                }
            } else {
                // All 3 bombs exist — relocate one at random
                let mut rng = rand::rng();
                let idx = rng.random_range(0..bombs.len());
                if let Some(pos) = generate_bomb_position(
                    board_width,
                    board_height,
                    &host_snake,
                    &guest_snake,
                    &bombs,
                    food,
                ) {
                    bombs[idx] = pos;
                }
            }
            last_bomb_tick = tick;
        }

        // Handle deaths with lives/respawn
        if host_died || guest_died {
            let host_on_last_life = host_died && host_snake.lives <= 1;
            let guest_on_last_life = guest_died && guest_snake.lives <= 1;

            if host_on_last_life || guest_on_last_life {
                // Game over — at least one player has no lives left
                let final_snakes = build_snakes_state(you, opponent, &host_snake, &guest_snake);

                let (host_result, guest_result) = if host_on_last_life && guest_on_last_life {
                    (MatchResult::Draw, MatchResult::Draw)
                } else if host_on_last_life {
                    (MatchResult::Loss, MatchResult::Win)
                } else {
                    (MatchResult::Win, MatchResult::Loss)
                };

                net.send_peer(&PeerMessage::GameOver {
                    result: guest_result,
                    reason: if guest_on_last_life {
                        guest_reason
                    } else {
                        GameOverReason::OpponentDied
                    },
                    final_snakes: final_snakes.clone(),
                })?;

                return Ok(HostGameResult {
                    result: host_result,
                    reason: if host_on_last_life {
                        host_reason
                    } else {
                        GameOverReason::OpponentDied
                    },
                    final_snakes,
                    disconnected: false,
                });
            }

            // Respawn snakes that died (they have lives remaining)
            if host_died {
                let (sx, sy) = find_respawn_position(
                    host_spawn.0,
                    host_spawn.1,
                    host_spawn.2,
                    &guest_snake,
                    board_width,
                    board_height,
                );
                host_snake.reset_at(sx, sy, host_spawn.2, 3);
                food =
                    generate_food_mp(board_width, board_height, &host_snake, &guest_snake, &bombs);
                food_placed_tick = tick;
            }
            if guest_died {
                let (sx, sy) = find_respawn_position(
                    guest_spawn.0,
                    guest_spawn.1,
                    guest_spawn.2,
                    &host_snake,
                    board_width,
                    board_height,
                );
                guest_snake.reset_at(sx, sy, guest_spawn.2, 3);
                food =
                    generate_food_mp(board_width, board_height, &host_snake, &guest_snake, &bombs);
                food_placed_tick = tick;
            }
        }

        // Check time expired
        if tick >= MP_GAME_DURATION_TICKS {
            let final_snakes = build_snakes_state(you, opponent, &host_snake, &guest_snake);

            let (host_result, guest_result) = if host_snake.score > guest_snake.score {
                (MatchResult::Win, MatchResult::Loss)
            } else if guest_snake.score > host_snake.score {
                (MatchResult::Loss, MatchResult::Win)
            } else {
                (MatchResult::Draw, MatchResult::Draw)
            };

            net.send_peer(&PeerMessage::GameOver {
                result: guest_result,
                reason: GameOverReason::TimeExpired,
                final_snakes: final_snakes.clone(),
            })?;

            return Ok(HostGameResult {
                result: host_result,
                reason: GameOverReason::TimeExpired,
                final_snakes,
                disconnected: false,
            });
        }

        // Send tick to guest
        let time_remaining = compute_time_remaining(tick);
        let snakes_state = build_snakes_state(you, opponent, &host_snake, &guest_snake);
        net.send_peer(&PeerMessage::GameTick {
            snakes: snakes_state,
            food,
            bombs: bombs.clone(),
            tick,
            time_remaining,
        })?;
    }
}

fn compute_time_remaining(tick: u64) -> u16 {
    let remaining_ticks = MP_GAME_DURATION_TICKS.saturating_sub(tick);
    // Convert ticks to seconds (10 ticks per second at 100ms/tick)
    (remaining_ticks / 10) as u16
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
            lives: host_snake.lives,
        },
        SnakeState {
            player: opponent.clone(),
            body: guest_snake.body.iter().copied().collect(),
            direction: guest_snake.direction,
            alive: guest_snake.alive,
            score: guest_snake.score,
            lives: guest_snake.lives,
        },
    ]
}

/// Find a safe respawn position. Prefers the original spawn point if the 3-cell spawn area
/// is free of the opponent's body. Otherwise picks a random open position.
fn find_respawn_position(
    preferred_x: u16,
    preferred_y: u16,
    direction: KeyDirection,
    opponent: &Snake,
    board_width: u16,
    board_height: u16,
) -> (u16, u16) {
    // Check if the 3-cell spawn area at preferred position is clear
    if spawn_area_clear(preferred_x, preferred_y, direction, opponent) {
        return (preferred_x, preferred_y);
    }

    // Try random positions
    let mut rng = rand::rng();
    for _ in 0..100 {
        let x = rng.random_range(0..board_width);
        let y = rng.random_range(0..board_height);
        if spawn_area_clear(x, y, direction, opponent) {
            return (x, y);
        }
    }

    // Fallback: return preferred position anyway (very unlikely to reach here)
    (preferred_x, preferred_y)
}

/// Check if a 3-cell snake spawn area (head + 2 body segments behind) is free of the opponent.
fn spawn_area_clear(head_x: u16, head_y: u16, direction: KeyDirection, opponent: &Snake) -> bool {
    let (dx, dy): (i16, i16) = match direction {
        KeyDirection::Right => (-1, 0),
        KeyDirection::Left => (1, 0),
        KeyDirection::Up => (0, 1),
        KeyDirection::Down => (0, -1),
    };
    for i in 0..3i16 {
        let px = (head_x as i16 + dx * i) as u16;
        let py = (head_y as i16 + dy * i) as u16;
        let pt = SpacePoint { x: px, y: py };
        if opponent.body.contains(&pt) {
            return false;
        }
    }
    true
}

fn generate_food_mp(
    width: u16,
    height: u16,
    snake1: &Snake,
    snake2: &Snake,
    bombs: &[SpacePoint],
) -> SpacePoint {
    let mut rng = rand::rng();
    loop {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        let pt = SpacePoint { x, y };
        if !snake1.body.contains(&pt) && !snake2.body.contains(&pt) && !bombs.contains(&pt) {
            return pt;
        }
    }
}

/// Generate a position for a bomb that doesn't overlap snakes, existing bombs, or food.
fn generate_bomb_position(
    width: u16,
    height: u16,
    snake1: &Snake,
    snake2: &Snake,
    bombs: &[SpacePoint],
    food: SpacePoint,
) -> Option<SpacePoint> {
    let mut rng = rand::rng();
    for _ in 0..100 {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        let pt = SpacePoint { x, y };
        if pt != food
            && !snake1.body.contains(&pt)
            && !snake2.body.contains(&pt)
            && !bombs.contains(&pt)
        {
            return Some(pt);
        }
    }
    None
}
