#[derive(Clone, Copy, PartialEq)]
pub struct SpacePoint {
    pub x: u16,
    pub y: u16,
}

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
