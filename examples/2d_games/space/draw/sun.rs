//! Солнце: корона, лучи, ядро и спрайт.

use std::f32::consts::TAU;

use frglib::{Color, Draw, Image, Object, Rect, Vec2};

use super::cull::body_visible;
use super::glow::draw_marker_rings;
use super::poly::fill_disc;

/// Корона Солнца: несколько мягких слоёв и медленно вращающиеся лучи.
///
/// Непрозрачное ядро рисуется отдельно, уже в отсортированной сцене: так
/// Солнце может закрывать планеты, ушедшие за него.
fn draw_corona(d: &mut Draw, center: Vec2, radius: f32, elapsed: f32) -> Result<(), frglib::Error> {
    let pulse = (elapsed * 1.6).sin() * 0.5 + 0.5;

    // От большого и почти прозрачного к маленькому и яркому. Раньше было
    // наоборот, и внешние слои гасили внутренние.
    for (scale, alpha) in [
        (2.45, 8),
        (2.05, 12),
        (1.72, 18),
        (1.46, 26),
        (1.24, 40),
        (1.10, 64),
    ] {
        let radius = radius * (scale + pulse * 0.04);

        fill_disc(d, center, radius, Color::rgba(255, 184, 92, alpha))?;
    }

    // Лучи: один вызов на все треугольники.
    const RAYS: usize = 22;

    let mut points = Vec::with_capacity(RAYS * 3);
    let mut indices = Vec::with_capacity(RAYS * 3);

    for ray in 0..RAYS {
        let angle = elapsed * 0.12 + TAU * ray as f32 / RAYS as f32;
        let flicker = ((elapsed * 1.9 + ray as f32 * 1.7).sin() * 0.5 + 0.5).powi(2);
        let length = radius * (1.30 + 0.45 * flicker);
        let half_width = 0.012 + 0.010 * flicker;

        let base = points.len() as i32;

        for offset in [-half_width, half_width] {
            let edge = angle + offset;
            points.push((
                center.x + edge.cos() * radius * 0.96,
                center.y + edge.sin() * radius * 0.96,
            ));
        }

        points.push((
            center.x + angle.cos() * length,
            center.y + angle.sin() * length,
        ));

        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    let ray_color = Color::rgba(255, 205, 120, (18.0 + pulse * 22.0) as u8);

    d.fill_object_polygon(
        &Object::new(center, Vec2::ZERO),
        &points,
        &indices,
        ray_color,
    )
}

/// Солнце в отсортированной сцене: ореол, непрозрачное ядро и спрайт.
#[allow(clippy::too_many_arguments)]
pub fn draw_sun_body(
    d: &mut Draw,
    center: Vec2,
    sun_size: f32,
    elapsed: f32,
    zoom: f32,
    sun_image: &Image,
    hovered: Option<&str>,
    selected: Option<&str>,
    view_rect: Rect,
) -> Result<(), frglib::Error> {
    let radius = sun_size * 0.5;

    if !body_visible(view_rect, center, radius) {
        return Ok(());
    }

    // Ореол и лучи, затем непрозрачное ядро: оно должно закрывать тела,
    // которые находятся за Солнцем.
    draw_corona(d, center, radius, elapsed)?;
    fill_disc(d, center, radius * 0.99, Color::rgba(255, 224, 150, 255))?;

    let object = Object::new(center, Vec2::new(sun_size, sun_size)).with_rotation(elapsed * 20.0);

    d.fill_textured_object(&object, sun_image)?;

    draw_marker_rings(d, "Sun", center, radius, zoom, hovered, selected, elapsed)
}
