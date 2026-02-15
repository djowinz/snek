use std::collections::VecDeque;

use crate::types::{KeyDirection, SpacePoint};

pub struct Snake {
    pub body: VecDeque<SpacePoint>,
    pub direction: KeyDirection,
    pub alive: bool,
    pub score: u16,
}

impl Snake {
    pub fn new(width: u16, height: u16) -> Self {
        let mut body = VecDeque::new();
        body.push_front(SpacePoint {
            x: (width / 2) - 2,
            y: height / 2,
        });
        body.push_front(SpacePoint {
            x: (width / 2) - 1,
            y: height / 2,
        });
        body.push_front(SpacePoint {
            x: width / 2,
            y: height / 2,
        });

        Self {
            body,
            direction: KeyDirection::Right,
            alive: true,
            score: 0,
        }
    }

    /// Advance the snake one step in its current direction, wrapping at board edges.
    /// Returns the new head position so the caller can check food/cross-snake collisions.
    pub fn advance(&mut self, width: u16, height: u16) -> SpacePoint {
        let head = self.body.front().expect("snake has no body");
        let mut new_x = head.x;
        let mut new_y = head.y;

        match self.direction {
            KeyDirection::Up => {
                if new_y == 0 {
                    new_y = height - 1;
                } else {
                    new_y -= 1;
                }
            }
            KeyDirection::Down => {
                if new_y == height - 1 {
                    new_y = 0;
                } else {
                    new_y += 1;
                }
            }
            KeyDirection::Left => {
                if new_x == 0 {
                    new_x = width - 1;
                } else {
                    new_x -= 1;
                }
            }
            KeyDirection::Right => {
                if new_x == width - 1 {
                    new_x = 0;
                } else {
                    new_x += 1;
                }
            }
        }

        let new_head = SpacePoint { x: new_x, y: new_y };

        // Self-collision check
        if self.body.contains(&new_head) {
            self.alive = false;
        }

        self.body.push_front(new_head);
        new_head
    }
}
