use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Fixed multiplayer board (play area, excluding border). Fits in standard 80x24 terminal.
pub const MP_BOARD_WIDTH: u16 = 60;
pub const MP_BOARD_HEIGHT: u16 = 20;
pub const MP_MIN_TERM_WIDTH: u16 = MP_BOARD_WIDTH + 2; // 62
pub const MP_MIN_TERM_HEIGHT: u16 = MP_BOARD_HEIGHT + 2; // 22

// Multiplayer game duration: 1200 ticks at 100ms/tick = 2 minutes
pub const MP_GAME_DURATION_TICKS: u64 = 1200;

// Bombs: spawn one every 3 seconds, max 3 on board, relocate once full
pub const MAX_BOMBS: usize = 3;
pub const BOMB_SPAWN_INTERVAL_TICKS: u64 = 30;
pub const BOMB_MOVE_INTERVAL_TICKS: u64 = 30;

// Security limits
pub const MAX_NAME_LENGTH: usize = 16;
pub const MAX_WS_MESSAGE_SIZE: usize = 65536; // 64 KB — room for large game states + PeerEnvelope wrapping
pub const MAX_WS_FRAME_SIZE: usize = 65536;
pub const RELAY_CHANNEL_BOUND: usize = 256;
pub const MAX_RELAY_CONNECTIONS: usize = 100;

// Rate limiting: 50 burst, refill 1 per 33ms ≈ 30 msg/sec sustained
// Host sends ~10 GameTick/sec; this gives 3x headroom for timing jitter + other messages
pub const RATE_LIMIT_BURST: u32 = 50;
pub const RATE_LIMIT_REFILL_MS: u64 = 33;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub struct SpacePoint {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum KeyDirection {
    Up,
    Down,
    Left,
    Right,
}

impl std::fmt::Display for KeyDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyDirection::Up => write!(f, "Up"),
            KeyDirection::Down => write!(f, "Down"),
            KeyDirection::Left => write!(f, "Left"),
            KeyDirection::Right => write!(f, "Right"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum SnakeColor {
    Blue,
    Magenta,
    Cyan,
    Yellow,
    Green,
    Red,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct PlayerInfo {
    pub id: Uuid,
    pub name: String,
    pub color: SnakeColor,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct SnakeState {
    pub player: PlayerInfo,
    pub body: Vec<SpacePoint>,
    pub direction: KeyDirection,
    pub alive: bool,
    pub score: u16,
    pub lives: u8,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum GameOverReason {
    SelfCollision,
    HeadOnCollisionLoss,
    BodyCollision,
    OpponentDisconnected,
    OpponentDied,
    TimeExpired,
    BombExplosion,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum MatchResult {
    Win,
    Loss,
    Draw,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum PeerRole {
    Host,
    Guest,
}
