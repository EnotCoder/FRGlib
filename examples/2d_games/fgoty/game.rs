//! Состояние мира: где стоит фрогти, куда смотрит и как шагает.

use frglib::{Object, TileMap, Vec2};

use crate::map::{Tile, TILE};
use crate::player::{Facing, SPEED};

/// Фрогти и всё, что меняется от кадра к кадру.
pub struct World {
    pub player: Object,
    pub facing: Facing,
    step: u32,
}

impl World {
    pub fn new() -> Self {
        World {
            player: Object::from_xywh(TILE * 2.5, TILE * 2.5, TILE, TILE),
            facing: Facing::Down,
            step: 0,
        }
    }

    /// Двигает фрогти и возвращает смещение для покачивания при ходьбе.
    pub fn update(&mut self, map: &TileMap<Tile>, direction: Vec2, dt: f32) -> f32 {
        // Пробуем шагнуть и проверяем точку, в которую приземлились: в воду и за
        // край карты не ходим, упираемся в границу вплотную.
        //
        // Проверяется только тот угол, в сторону которого идём: у нуля в Rust
        // знак равен единице, так что через `signum` проверка уехала бы на
        // пол-объекта в сторону, куда движения нет.
        let half = self.player.size * 0.5;
        let lead = Vec2::new(
            match direction.x {
                x if x > 0.0 => half.x,
                x if x < 0.0 => -half.x,
                _ => 0.0,
            },
            match direction.y {
                y if y > 0.0 => half.y,
                y if y < 0.0 => -half.y,
                _ => 0.0,
            },
        );

        let target = self.player.position + direction * SPEED * dt;
        if direction == Vec2::ZERO
            || map
                .get_at(target + lead)
                .is_some_and(|tile| tile.walkable())
        {
            self.player.position = target;
        }
        self.player.velocity = direction * SPEED;

        if let Some(next) = Facing::of(direction) {
            self.facing = next;
        }

        // Покачивание при ходьбе: кадров анимации нет, так что просто качаем.
        let moving = direction != Vec2::ZERO;
        if moving {
            self.step += 1;
        }

        if moving && (self.step / 7) % 2 == 0 {
            3.0
        } else {
            0.0
        }
    }
}
