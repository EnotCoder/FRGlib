//! Перевод мировых координат в экранные и подбор стартового зума.

use frglib::{Camera, Vec2};

use crate::consts::{FIT_MARGIN, ORBIT_TILT, SYSTEM_RADIUS};

/// Зум, при котором система целиком влезает в окно.
pub fn fit_zoom(size: (u32, u32)) -> f32 {
    let half = Vec2::new(size.0 as f32, size.1 as f32) * 0.5;

    (half.x / SYSTEM_RADIUS).min(half.y / (SYSTEM_RADIUS * ORBIT_TILT)) * FIT_MARGIN
}

/// Экранная точка, в которую попадает мировая координата.
pub fn world_to_screen(camera: Camera, world: Vec2, window: Vec2) -> Vec2 {
    (world - camera.position) * camera.zoom + window * 0.5
}
