//! Отрисовка планеты: кольца, свечение, спрайт и ночная сторона.

use frglib::{Color, Draw, Object, Rect, Vec2};

use crate::bodies::{Planet, PlanetPose};

use super::cull::body_visible;
use super::glow::{draw_glow, draw_marker_rings, glow_color};
use super::night::draw_night_side;
use super::poly::fill_ellipse_ring_half;

/// Кольца Сатурна: задняя половина до планеты, передняя -- после.
fn draw_saturn_rings(
    d: &mut Draw,
    center: Vec2,
    radius: f32,
    front: bool,
) -> Result<(), frglib::Error> {
    fill_ellipse_ring_half(
        d,
        center,
        radius * 1.35,
        radius * 1.70,
        0.30,
        front,
        Color::rgba(206, 186, 142, 170),
    )?;

    fill_ellipse_ring_half(
        d,
        center,
        radius * 1.72,
        radius * 2.10,
        0.30,
        front,
        Color::rgba(188, 164, 124, 120),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn draw_planet(
    d: &mut Draw,
    planet: &Planet,
    pose: &PlanetPose,
    elapsed: f32,
    center: Vec2,
    zoom: f32,
    view_rect: Rect,
    hovered: Option<&str>,
    selected: Option<&str>,
) -> Result<(), frglib::Error> {
    let radius = planet.radius();

    if !body_visible(view_rect, pose.position, radius) {
        return Ok(());
    }

    let is_saturn = planet.name == "Saturn";

    // Задняя половина колец -- до планеты, передняя -- после.
    if is_saturn {
        draw_saturn_rings(d, pose.position, radius, false)?;
    }

    draw_glow(
        d,
        pose.position,
        radius,
        zoom,
        glow_color(planet.name, hovered, selected, elapsed),
    )?;

    let object = Object::new(pose.position, Vec2::new(planet.size, planet.size))
        .with_rotation(elapsed * planet.speed * 80.0);

    match &planet.texture {
        Some(image) => d.fill_textured_object(&object, image)?,
        None => d.fill_object(&object, planet.color)?,
    }

    draw_night_side(d, center, pose.position, radius)?;

    if is_saturn {
        draw_saturn_rings(d, pose.position, radius, true)?;
    }

    draw_marker_rings(
        d,
        planet.name,
        pose.position,
        radius,
        zoom,
        hovered,
        selected,
        elapsed,
    )
}
