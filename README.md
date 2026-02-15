# snek-rs
![snek-demo](https://github.com/user-attachments/assets/3a8bb5b7-4b15-4edf-9019-63d4d5961b5b)

A terminal Snake game with networked multiplayer support, built in Rust.

Play solo against yourself or compete head-to-head with another player over WebSocket. The TUI runs in any terminal that supports 256 colors.

## Features

- **Single-player** -- classic Snake with pause, restart, and game-over menus
- **Multiplayer** -- real-time 1v1 matches over a lightweight relay server
  - Matchmaking queue with animated search screen
  - 2-minute timed rounds with countdown timer
  - Lives system (2 extra lives per player, respawn on death)
  - Bombs that spawn during the match, disguised as green `B` among green `#` food -- eating one shrinks your snake and costs a point
  - Score-based winner at timeout, or elimination on final death
- **Fixed multiplayer board** (60x20) centered in the terminal so snakes never disappear off-screen
- **Color-coded snakes** with per-player names and scores in the border

## Requirements

- **Rust** 2024 edition (1.85+)
- A terminal at least **62 columns x 22 rows** for multiplayer (single-player adapts to terminal size)

## Quick Start

```sh
# Build everything
cargo build --workspace

# Start the relay server (multiplayer)
cargo run -p snek-relay

# In another terminal, start the client
cargo run -p snek-client
```

To connect to a relay on a different host:

```sh
cargo run -p snek-client -- ws://192.168.1.50:9090
```

The default relay address is `ws://127.0.0.1:9090`.

## How to Play

### Controls

| Key              | Action              |
|------------------|----------------------|
| Arrow keys / WASD | Move snake           |
| Enter            | Confirm selection    |
| Esc              | Pause / Back / Quit  |

### Single-Player

Classic Snake. Eat food (`#`) to grow and score points. Colliding with yourself ends the game. The board wraps at the edges.

### Multiplayer

1. Enter your name (1-16 alphanumeric characters)
2. Select **Multiplayer** from the menu
3. Wait for an opponent to connect
4. After a 3-2-1 countdown, the match begins

**Scoring:**
- Eat food (`#`) to grow and gain a point
- Avoid bombs (`B`) -- they look like food but shrink your snake and cost a point
- Colliding with the opponent's body or losing a head-on collision costs a life
- Each player starts with 2 extra lives; losing all lives eliminates you
- If time runs out, the player with the higher score wins

**Bombs:**
- A new bomb spawns every 3 seconds, up to 3 on the board
- Once 3 bombs exist, they start relocating to new random positions
- Bombs are green like food to make them tricky to spot (`B` vs `#`)
- Eating a bomb when your snake is only a head kills you (costs a life)

## Architecture

The project is a Cargo workspace with three crates:

```
Cargo.toml                  # workspace root
crates/
  snek-shared/              # shared types + message protocol
  snek-relay/               # matchmaking + WebSocket relay server
  snek-client/              # TUI client (single-player + multiplayer)
```

### snek-shared

Serializable types and the message protocol used by both client and relay:

- **Types** -- `SpacePoint`, `KeyDirection`, `SnakeColor`, `PlayerInfo`, `SnakeState`, `GameOverReason`, `MatchResult`, `PeerRole`
- **Messages** -- `ClientRelayMessage` (client to relay), `ServerRelayMessage` (relay to client), `PeerMessage` (host to guest, forwarded by relay in `PeerEnvelope`)
- **Constants** -- board dimensions, game duration, bomb config, security limits, rate limiting

### snek-relay

A lightweight async WebSocket relay server that binds to `127.0.0.1:9090`. It performs no game logic -- it only handles:

- **Matchmaking** (`lobby.rs`) -- queues players and pairs them, assigning Host/Guest roles and random snake colors
- **Message forwarding** (`session.rs`) -- bidirectional relay of `PeerMessage` between paired peers via `PeerEnvelope` wrapping
- **Security** -- connection limits (max 100), rate limiting (30 msg/sec sustained with 50 burst), configurable WebSocket frame/message sizes

### snek-client

The TUI client handles both game modes:

- **State machine** (`app.rs`) -- drives the flow: Name Input -> Mode Select -> Single-Player or Multiplayer (Connect -> Search -> Countdown -> Game -> Game Over)
- **Single-player** (`game/single_player.rs`) -- synchronous local game loop with pause menu
- **Multiplayer host** (`game/host.rs`) -- runs authoritative game logic (movement, collisions, food, bombs, timer, lives/respawn) and sends `GameTick` to guest
- **Multiplayer guest** (`game/guest.rs`) -- sends directional input to host, renders received state
- **Networking** (`network.rs`) -- async WebSocket connection with mpsc channels bridging async I/O to the synchronous game loop
- **UI widgets** (`ui/`) -- ratatui widgets for the game boards, pause/game-over menus, and terminal-too-small overlay
- **Screens** (`screens/`) -- name input, mode selection, animated searching, countdown, multiplayer game-over

### Multiplayer Design

The multiplayer model is peer-to-peer with a thin relay:

```
  Host Client                 Relay                Guest Client
  +-----------+          +-------------+          +-----------+
  | game logic| --tick-->| PeerEnvelope|--tick--> | render    |
  | render    | <--dir---| forwarding  |<--dir--- | input     |
  +-----------+          +-------------+          +-----------+
```

- The **host** runs all game logic: snake movement, collision detection, food/bomb spawning, scoring, timer, and lives. It sends a `GameTick` message (~10/sec) containing full game state.
- The **guest** sends `ChangeDirection` messages and renders whatever the host sends.
- The **relay** wraps each peer's messages in a `PeerEnvelope` and forwards them to the other peer. It has no knowledge of game rules.

## Development

```sh
# Type-check without building
cargo check --workspace

# Lint
cargo clippy --workspace

# Format
cargo fmt --all

# Format check (CI)
cargo fmt --all -- --check
```

No tests exist yet.

## Dependencies

| Crate                | Purpose                         |
|----------------------|---------------------------------|
| `ratatui`            | TUI framework (widgets, layout) |
| `crossterm`          | Terminal backend (input, raw mode) |
| `tokio`              | Async runtime                   |
| `tokio-tungstenite`  | Async WebSocket client/server   |
| `futures-util`       | Stream/Sink utilities           |
| `serde` + `serde_json` | Message serialization        |
| `uuid`               | Player and game IDs             |
| `rand`               | Food/bomb placement RNG         |
| `color-eyre`         | Error handling and reporting    |

## License

This project does not currently specify a license.
