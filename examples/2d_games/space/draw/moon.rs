//! Отрисовка спутника.

use frglib::{Draw, Object, Rect, Vec2};

use crate::bodies::Moon;

use super::cull::body_visible;
use super::glow::{draw_glow, draw_marker_rings, glow_color};
use super::night::draw_night_side;

#[allow(clippy::too_many_arguments)]
pub fn draw_moon(
    d: &mut Draw,
    moon: &Moon,
    position: Vec2,
    sun: Vec2,
    zoom: f32,
    view_rect: Rect,
    hovered: Option<&str>,
    selected: Option<&str>,
    elapsed: f32,
) -> Result<(), frglib::Error> {
    let radius = moon.radius();

    if !body_visible(view_rect, position, radius) {
        return Ok(());
    }

    draw_glow(
        d,
        position,
        radius,
        zoom,
        glow_color(moon.name, hovered, selected, elapsed),
    )?;

    let moon_object = Object::new(position, Vec2::new(moon.size, moon.size));

    match &moon.texture {
        Some(image) => d.fill_textured_object(&moon_object, image)?,
        None => d.fill_object(&moon_object, moon.color)?,
    }

    draw_night_side(d, sun, position, radius)?;

    draw_marker_rings(
        d, moon.name, position, radius, zoom, hovered, selected, elapsed,
    )
}
