use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::DefaultTerminal;

use crate::network::NetworkConnection;
use crate::ui::board_mp::MultiplayerBoard;
use crate::ui::terminal_warning::TerminalTooSmallOverlay;
use snek_shared::*;

#[allow(dead_code)]
pub struct GuestGameResult {
    pub result: MatchResult,
    pub reason: GameOverReason,
    pub final_snakes: Vec<SnakeState>,
    pub disconnected: bool,
}

pub fn run_guest(
    terminal: &mut DefaultTerminal,
    net: &mut NetworkConnection,
    board_width: u16,
    board_height: u16,
    you: &PlayerInfo,
    opponent: &PlayerInfo,
) -> Result<GuestGameResult> {
    // Wait for countdown messages from host
    loop {
        // Drain network
        if let Ok(msg) = net.rx.try_recv()
            && let Ok(relay_msg) = serde_json::from_str::<ServerRelayMessage>(&msg)
        {
            match relay_msg {
                ServerRelayMessage::PeerEnvelope { payload } => {
                    if let Ok(peer_msg) = serde_json::from_str::<PeerMessage>(&payload) {
                        match peer_msg {
                            PeerMessage::Countdown { remaining } => {
                                terminal.draw(|frame| {
                                    let screen = crate::screens::countdown::CountdownScreen {
                                        remaining,
                                        you,
                                        opponent,
                                    };
                                    frame.render_widget(&screen, frame.area());
                                })?;
                                if remaining == 1 {
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                    break;
                                }
                            }
                            PeerMessage::GameTick { .. } => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                ServerRelayMessage::OpponentDisconnected => {
                    return Ok(GuestGameResult {
                        result: MatchResult::Win,
                        reason: GameOverReason::OpponentDisconnected,
                        final_snakes: Vec::new(),
                        disconnected: true,
                    });
                }
                _ => {}
            }
        }

        // Poll input during countdown (discard)
        if crossterm::event::poll(std::time::Duration::from_millis(16))? {
            let _ = event::read()?;
        }
    }

    // Main game loop - render state from host, send input
    let mut last_snakes: Vec<SnakeState> = Vec::new();
    let mut last_food = SpacePoint { x: 0, y: 0 };
    let mut last_bombs: Vec<SpacePoint> = Vec::new();
    let mut last_time_remaining: u16 = (MP_GAME_DURATION_TICKS / 10) as u16;
    let mut last_direction: Option<KeyDirection> = None;
    let mut terminal_too_small = {
        let dim = terminal.size()?;
        dim.width < MP_MIN_TERM_WIDTH || dim.height < MP_MIN_TERM_HEIGHT
    };

    loop {
        // Render latest state
        if !last_snakes.is_empty() {
            let too_small = terminal_too_small;
            let time_remaining = last_time_remaining;
            let bombs = &last_bombs;
            terminal.draw(|frame| {
                let board = MultiplayerBoard {
                    snakes: &last_snakes,
                    food: last_food,
                    bombs,
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
        }

        // Poll input (short timeout for responsiveness)
        if crossterm::event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    let new_dir = match key.code {
                        KeyCode::Up | KeyCode::Char('w') => Some(KeyDirection::Up),
                        KeyCode::Down | KeyCode::Char('s') => Some(KeyDirection::Down),
                        KeyCode::Left | KeyCode::Char('a') => Some(KeyDirection::Left),
                        KeyCode::Right | KeyCode::Char('d') => Some(KeyDirection::Right),
                        KeyCode::Esc => {
                            net.send_peer(&PeerMessage::LeaveGame)?;
                            return Ok(GuestGameResult {
                                result: MatchResult::Loss,
                                reason: GameOverReason::OpponentDisconnected,
                                final_snakes: last_snakes,
                                disconnected: true,
                            });
                        }
                        _ => None,
                    };

                    if let Some(dir) = new_dir
                        && last_direction != Some(dir)
                    {
                        net.send_peer(&PeerMessage::ChangeDirection { direction: dir })?;
                        last_direction = Some(dir);
                    }
                }
                Event::Resize(w, h) => {
                    terminal_too_small = w < MP_MIN_TERM_WIDTH || h < MP_MIN_TERM_HEIGHT;
                }
                _ => {}
            }
        }

        // Drain network messages
        while let Ok(msg) = net.rx.try_recv() {
            if let Ok(relay_msg) = serde_json::from_str::<ServerRelayMessage>(&msg) {
                match relay_msg {
                    ServerRelayMessage::PeerEnvelope { payload } => {
                        if let Ok(peer_msg) = serde_json::from_str::<PeerMessage>(&payload) {
                            match peer_msg {
                                PeerMessage::GameTick {
                                    snakes,
                                    food,
                                    bombs,
                                    time_remaining,
                                    ..
                                } => {
                                    // Validate received state before applying
                                    let valid = food.x < board_width
                                        && food.y < board_height
                                        && bombs
                                            .iter()
                                            .all(|b| b.x < board_width && b.y < board_height)
                                        && snakes.iter().all(|s| {
                                            s.player.name.len() <= MAX_NAME_LENGTH
                                                && s.body.iter().all(|p| {
                                                    p.x < board_width && p.y < board_height
                                                })
                                        });
                                    if valid {
                                        last_snakes = snakes;
                                        last_food = food;
                                        last_bombs = bombs;
                                        last_time_remaining = time_remaining;
                                    }
                                }
                                PeerMessage::GameOver {
                                    result,
                                    reason,
                                    final_snakes,
                                } => {
                                    return Ok(GuestGameResult {
                                        result,
                                        reason,
                                        final_snakes,
                                        disconnected: false,
                                    });
                                }
                                PeerMessage::Countdown { .. } => {
                                    // Late countdown message, ignore
                                }
                                _ => {}
                            }
                        }
                    }
                    ServerRelayMessage::OpponentDisconnected => {
                        return Ok(GuestGameResult {
                            result: MatchResult::Win,
                            reason: GameOverReason::OpponentDisconnected,
                            final_snakes: last_snakes,
                            disconnected: true,
                        });
                    }
                    _ => {}
                }
            }
        }
    }
}
