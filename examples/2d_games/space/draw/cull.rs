//! Отсечение того, что не попало в кадр.

use frglib::{Rect, Vec2};

use crate::consts::ORBIT_TILT;

/// Считает, попадает ли эллипс орбиты в кадр.
pub fn orbit_visible(view_rect: Rect, center: Vec2, radius: f32) -> bool {
    let rect = Rect::new(
        center.x - radius,
        center.y - radius * ORBIT_TILT,
        radius * 2.0,
        radius * 2.0 * ORBIT_TILT,
    );

    view_rect.intersects(rect)
}

/// Считает, попадает ли диск тела в кадр.
pub fn body_visible(view_rect: Rect, position: Vec2, radius: f32) -> bool {
    let rect = Rect::new(
        position.x - radius,
        position.y - radius,
        radius * 2.0,
        radius * 2.0,
    );

    view_rect.intersects(rect)
}
