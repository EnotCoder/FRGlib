//! Свечение тел и обводка выбора.
//!
//! Всё рисуется в том же проходе, что и тело: ближний объект должен
//! перекрывать ореол дальнего.

use std::f32::consts::TAU;

use frglib::{Color, Draw, Object, Vec2};

use super::poly::fill_disc;

const GLOW_IDLE: Color = Color::rgba(130, 175, 255, 40);
const GLOW_SELECTED: Color = Color::rgba(155, 210, 255, 110);
const RING_SELECTED: Color = Color::rgba(155, 210, 255, 170);

/// Рисует мягкое свечение вокруг тела.
///
/// Слои идут от большого и прозрачного к маленькому и яркому. Раньше было
/// наоборот, и внешние круги гасили внутренние.
pub fn draw_glow(
    d: &mut Draw,
    center: Vec2,
    radius: f32,
    zoom: f32,
    color: Color,
) -> Result<(), frglib::Error> {
    let margin = 2.5 / zoom.max(0.001);

    for (scale, alpha_scale) in [
        (2.6, 0.05),
        (2.1, 0.08),
        (1.75, 0.13),
        (1.45, 0.20),
        (1.20, 0.33),
        (1.02, 0.60),
    ] {
        let alpha = (color.a as f32 * alpha_scale).clamp(0.0, 255.0) as u8;

        fill_disc(
            d,
            center,
            radius + margin * scale,
            Color::rgba(color.r, color.g, color.b, alpha),
        )?;
    }

    Ok(())
}

/// Цвет свечения тела: обычное, выбранное или подсвеченное курсором.
pub fn glow_color(
    name: &str,
    hovered: Option<&str>,
    selected: Option<&str>,
    elapsed: f32,
) -> Color {
    if hovered == Some(name) {
        let pulse = (elapsed * 6.0).sin() * 0.5 + 0.5;

        Color::rgba(185, 225, 255, (150.0 + pulse * 70.0) as u8)
    } else if selected == Some(name) {
        GLOW_SELECTED
    } else {
        GLOW_IDLE
    }
}

/// Обводка выбранного и подсвеченного тела. Рисуется сразу после самого
/// тела, поэтому более близкие объекты естественно перекрывают её.
#[allow(clippy::too_many_arguments)]
pub fn draw_marker_rings(
    d: &mut Draw,
    name: &str,
    position: Vec2,
    radius: f32,
    zoom: f32,
    hovered: Option<&str>,
    selected: Option<&str>,
    elapsed: f32,
) -> Result<(), frglib::Error> {
    if selected == Some(name) {
        draw_ring(d, position, radius, zoom, RING_SELECTED)?;
    }

    if hovered == Some(name) {
        let pulse = (elapsed * 6.0).sin() * 0.5 + 0.5;

        draw_ring(
            d,
            position,
            radius,
            zoom,
            Color::rgba(205, 235, 255, (150.0 + pulse * 80.0) as u8),
        )?;
    }

    Ok(())
}

/// Рисует тонкое кольцо вокруг тела.
fn draw_ring(
    d: &mut Draw,
    center: Vec2,
    radius: f32,
    zoom: f32,
    color: Color,
) -> Result<(), frglib::Error> {
    let ring_radius = radius + 3.0 / zoom.max(0.001);
    let points = (0..64)
        .map(|i| {
            let angle = TAU * i as f32 / 64.0;
            (
                center.x + angle.cos() * ring_radius,
                center.y + angle.sin() * ring_radius,
            )
        })
        .collect::<Vec<_>>();

    d.draw_object_polygon(&Object::new(center, Vec2::ZERO), &points, color)
}
