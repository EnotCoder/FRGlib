use frglib::{Color, Draw, Object, Vec2};
use std::f32::consts::TAU;

use super::poly::fill_disc;

/// Корона Солнца: несколько мягких слоёв и медленно вращающиеся лучи.
///
/// Рисуется до спрайта, поэтому виден только ореол вокруг диска.
pub fn draw_sun(
    d: &mut Draw,
    center: Vec2,
    radius: f32,
    elapsed: f32,
) -> Result<(), frglib::Error> {
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
    )?;

    // Ядро -- на случай, если спрайт полупрозрачный по краям.
    fill_disc(d, center, radius * 0.99, Color::rgba(255, 224, 150, 255))
}
