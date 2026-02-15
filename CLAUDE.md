# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

- **Build:** `cargo build --workspace`
- **Run client:** `cargo run -p snek-client`
- **Run relay:** `cargo run -p snek-relay`
- **Check (fast compile check):** `cargo check --workspace`
- **Lint:** `cargo clippy --workspace`
- **Format:** `cargo fmt --all`
- **Format check:** `cargo fmt --all -- --check`

No tests exist yet.

## Architecture

This is a terminal Snake game ("snek") with networked multiplayer support, organized as a Cargo workspace with three crates:

### Workspace Structure

```
Cargo.toml                    # workspace root
crates/
  snek-shared/                # shared types + message protocol
  snek-relay/                 # lightweight matchmaking + forwarding relay server
  snek-client/                # TUI client (single-player + multiplayer)
```

### snek-shared
Shared types and message protocol used by both client and relay:
- `types.rs` — `SpacePoint`, `KeyDirection`, `SnakeColor`, `PlayerInfo`, `SnakeState`, `GameOverReason`, `MatchResult`, `PeerRole`
- `messages.rs` — `ClientRelayMessage`, `ServerRelayMessage`, `PeerMessage` (all serde-tagged enums)

### snek-relay
Lightweight WebSocket relay server (binds 0.0.0.0:9090). No game logic:
- `main.rs` — TCP listener + WebSocket upgrade
- `lobby.rs` — matchmaking queue, player pairing, host/guest role assignment
- `session.rs` — bidirectional message forwarding between paired peers

### snek-client
TUI client with menu flow and both single-player and multiplayer modes:
- `main.rs` — entry point, inits terminal
- `app.rs` — state machine: NameInput → ModeSelect → SinglePlayer or Multiplayer flow
- `network.rs` — WebSocket connection with mpsc channels for send/recv
- `snake.rs` — `Snake` struct with movement, collision, wrapping logic
- `game/single_player.rs` — local game loop (synchronous, no networking)
- `game/host.rs` — host-side multiplayer loop (runs authoritative game logic, sends state)
- `game/guest.rs` — guest-side multiplayer loop (sends input, renders received state)
- `screens/` — `name_input`, `mode_select`, `searching`, `countdown`, `game_over_mp`
- `ui/board.rs` — single-player board Widget
- `ui/board_mp.rs` — multiplayer board Widget (2 colored snakes, scores)
- `ui/menu.rs` — `PauseMenu`, `GameOverMenu` widgets

### Multiplayer Design
Peer-to-peer with lightweight relay. One player is host (runs game logic), the other is guest (sends input, receives state). Both connect to the relay which handles matchmaking and message forwarding. The relay wraps peer messages in `PeerEnvelope` for forwarding.

## Dependencies

- `ratatui` + `crossterm` — TUI framework and terminal backend
- `color-eyre` — error handling
- `rand` — food placement RNG
- `serde` + `serde_json` — message serialization
- `tokio` + `tokio-tungstenite` + `futures-util` — async runtime + WebSocket
- `uuid` — player and game IDs

## Notes

- Rust 2024 edition
- The game board dimensions are derived from terminal size minus border (width-2, height-2)
- Multiplayer relay default address: `ws://127.0.0.1:9090`
