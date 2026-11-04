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
        let velocity = direction * SPEED;

        // Движение по X
        if velocity.x != 0.0 {
            let next_x = self.player.position.x + velocity.x * dt;
            let mut next_rect = self.player.rect();
            next_rect.x = next_x - next_rect.w / 2.0;
            next_rect.y = self.player.position.y - next_rect.h / 2.0;

            if !self.check_collision(map, next_rect) {
                self.player.position.x = next_x;
            }
        }

        // Движение по Y
        if velocity.y != 0.0 {
            let next_y = self.player.position.y + velocity.y * dt;
            let mut next_rect = self.player.rect();
            next_rect.x = self.player.position.x - next_rect.w / 2.0;
            next_rect.y = next_y - next_rect.h / 2.0;

            if !self.check_collision(map, next_rect) {
                self.player.position.y = next_y;
            }
        }

        self.player.velocity = velocity;
        
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

    fn check_collision(&self, map: &TileMap<Tile>, rect: frglib::Rect) -> bool {
        let tile_x = (rect.x / TILE).floor() as i32;
        let tile_y = (rect.y / TILE).floor() as i32;
        let tile_w = (rect.w / TILE).ceil() as i32 + 1;
        let tile_h = (rect.h / TILE).ceil() as i32 + 1;

        for dx in 0..=tile_w {
            for dy in 0..=tile_h {
                let tx = tile_x + dx;
                let ty = tile_y + dy;
                if tx >= 0 && ty >= 0 {
                    if let Some(tile) = map.get(tx as usize, ty as usize) {
                        if !tile.walkable() {
                            let wall_rect =
                                frglib::Rect::new(tx as f32 * TILE, ty as f32 * TILE, TILE, TILE);
                            if rect.overlaps(&wall_rect) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }
}
