//! Затемнение ночной стороны тела.

use frglib::{Color, Draw, Vec2};

use super::poly::fill_night_side;

/// Цвет ночной стороны: почти чёрный, с лёгкой синью.
const NIGHT: Color = Color::rgba(2, 4, 12, 112);

/// Затемняет половину диска, отвёрнутую от Солнца.
pub fn draw_night_side(
    d: &mut Draw,
    sun: Vec2,
    body: Vec2,
    radius: f32,
) -> Result<(), frglib::Error> {
    let direction = sun - body;

    if direction.length_squared() < f32::EPSILON {
        return Ok(());
    }

    let angle = direction.y.atan2(direction.x);

    fill_night_side(d, body, radius, angle, NIGHT)
}
