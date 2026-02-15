use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::*;

// --- Client → Relay ---

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientRelayMessage {
    Register { name: String },
    SearchMatch,
    CancelSearch,
    Disconnect,
}

// --- Relay → Client ---

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ServerRelayMessage {
    Registered {
        player: PlayerInfo,
    },
    SearchingMatch,
    SearchCancelled,
    MatchFound {
        game_id: Uuid,
        role: PeerRole,
        board_width: u16,
        board_height: u16,
        opponent: PlayerInfo,
        you: PlayerInfo,
    },
    OpponentDisconnected,
    Error {
        message: String,
    },
    PeerEnvelope {
        payload: String,
    },
}

// --- Host ↔ Guest (forwarded by relay inside PeerEnvelope) ---

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum PeerMessage {
    // Guest → Host
    ChangeDirection {
        direction: KeyDirection,
    },
    // Host → Guest
    Countdown {
        remaining: u8,
    },
    GameTick {
        snakes: Vec<SnakeState>,
        food: SpacePoint,
        tick: u64,
    },
    GameOver {
        result: MatchResult,
        reason: GameOverReason,
        final_snakes: Vec<SnakeState>,
    },
    // Either direction
    PlayAgain,
    LeaveGame,
}
