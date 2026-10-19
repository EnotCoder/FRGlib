//! Фрогти: куда смотрит и как быстро бежит.

use frglib::Vec2;

/// Скорость ходьбы, единиц мира в секунду.
pub const SPEED: f32 = 190.0;

/// Каким кадром смотрит фрогти.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Facing {
    Down,
    Left,
    Right,
    Up,
}

impl Facing {
    /// По направлению движения. Без движения остаётся то, что было.
    pub fn of(direction: Vec2) -> Option<Self> {
        if direction.y.abs() > direction.x.abs() {
            Some(if direction.y < 0.0 {
                Facing::Up
            } else {
                Facing::Down
            })
        } else if direction.x < 0.0 {
            Some(Facing::Left)
        } else if direction.x > 0.0 {
            Some(Facing::Right)
        } else {
            None
        }
    }

    pub fn index(self) -> usize {
        match self {
            Facing::Down => 0,
            Facing::Left => 1,
            Facing::Right => 2,
            Facing::Up => 3,
        }
    }

    pub fn file(self) -> &'static str {
        match self {
            Facing::Down => "fgoty_down.png",
            Facing::Left => "fgoty_left.png",
            Facing::Right => "fgoty_right.png",
            Facing::Up => "fgoty_up.png",
        }
    }
}
