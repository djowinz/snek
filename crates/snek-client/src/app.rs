use color_eyre::Result;
use ratatui::DefaultTerminal;

use crate::game::{guest, host, single_player};
use crate::network::NetworkConnection;
use crate::screens::game_over_mp::{self, GameOverMpChoice};
use crate::screens::mode_select::{self, ModeSelection};
use crate::screens::name_input::{self, NameInputResult};
use crate::screens::searching::{self, SearchResult};
use snek_shared::*;

pub fn run(mut terminal: DefaultTerminal, relay_url: &str) -> Result<()> {
    // Name input
    let player_name = match name_input::run_name_input(&mut terminal)? {
        NameInputResult::Name(name) => name,
        NameInputResult::Quit => return Ok(()),
    };

    // Main menu loop
    loop {
        match mode_select::run_mode_select(&mut terminal, &player_name)? {
            ModeSelection::SinglePlayer => {
                let dim = terminal.size()?;
                let game_state = single_player::GameState::new(dim.width - 2, dim.height - 2);
                single_player::run(&mut terminal, game_state)?;
            }
            ModeSelection::Multiplayer => {
                match run_multiplayer(&mut terminal, &player_name, relay_url) {
                    Ok(MultiplayerOutcome::QuitToMenu) => continue,
                    Ok(MultiplayerOutcome::Quit) => return Ok(()),
                    Err(_e) => {
                        continue;
                    }
                }
            }
            ModeSelection::Quit => return Ok(()),
        }
    }
}

enum MultiplayerOutcome {
    QuitToMenu,
    Quit,
}

fn wait_for_registered(net: &mut NetworkConnection) {
    loop {
        if let Ok(msg) = net.rx.try_recv()
            && let Ok(ServerRelayMessage::Registered { .. }) =
                serde_json::from_str::<ServerRelayMessage>(&msg)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn run_multiplayer(
    terminal: &mut DefaultTerminal,
    player_name: &str,
    relay_url: &str,
) -> Result<MultiplayerOutcome> {
    let rt = tokio::runtime::Runtime::new()?;

    let mut net = rt.block_on(async { NetworkConnection::connect(relay_url).await })?;

    net.send_relay(&ClientRelayMessage::Register {
        name: player_name.to_string(),
    })?;

    wait_for_registered(&mut net);

    loop {
        // Check terminal size before searching for a match
        let dim = terminal.size()?;
        if dim.width < MP_MIN_TERM_WIDTH || dim.height < MP_MIN_TERM_HEIGHT {
            return Ok(MultiplayerOutcome::QuitToMenu);
        }

        net.send_relay(&ClientRelayMessage::SearchMatch)?;

        let match_info = loop {
            match searching::run_searching(terminal, &mut net.rx)? {
                SearchResult::Cancelled => {
                    net.send_relay(&ClientRelayMessage::CancelSearch)?;
                    return Ok(MultiplayerOutcome::QuitToMenu);
                }
                SearchResult::MessageReceived(msg) => {
                    if let Ok(relay_msg) = serde_json::from_str::<ServerRelayMessage>(&msg) {
                        match relay_msg {
                            ServerRelayMessage::MatchFound {
                                role,
                                board_width,
                                board_height,
                                opponent,
                                you,
                                ..
                            } => {
                                break (role, board_width, board_height, opponent, you);
                            }
                            ServerRelayMessage::SearchingMatch => continue,
                            _ => continue,
                        }
                    }
                }
            }
        };

        let (role, board_width, board_height, opponent, you) = match_info;

        let (result, final_snakes, disconnected) = match role {
            PeerRole::Host => {
                let gr = host::run_host(
                    terminal,
                    &mut net,
                    board_width,
                    board_height,
                    &you,
                    &opponent,
                )?;
                (gr.result, gr.final_snakes, gr.disconnected)
            }
            PeerRole::Guest => {
                let gr = guest::run_guest(
                    terminal,
                    &mut net,
                    board_width,
                    board_height,
                    &you,
                    &opponent,
                )?;
                (gr.result, gr.final_snakes, gr.disconnected)
            }
        };

        if disconnected {
            return Ok(MultiplayerOutcome::QuitToMenu);
        }

        match game_over_mp::run_game_over_mp(terminal, result, &final_snakes)? {
            GameOverMpChoice::SearchAgain => {
                net = rt.block_on(async { NetworkConnection::connect(relay_url).await })?;
                net.send_relay(&ClientRelayMessage::Register {
                    name: player_name.to_string(),
                })?;
                wait_for_registered(&mut net);
                continue;
            }
            GameOverMpChoice::QuitToMenu => return Ok(MultiplayerOutcome::QuitToMenu),
            GameOverMpChoice::Quit => return Ok(MultiplayerOutcome::Quit),
        }
    }
}
