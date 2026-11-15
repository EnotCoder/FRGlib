use frglib::{Color, Draw, Vec2};

use crate::bodies::{Belt, SHADES, SHADE_BRIGHTNESS};
use crate::consts::ORBIT_TILT;

use super::poly::fill_quads;

/// Рисует частицы пояса, вращающиеся вокруг Солнца.
///
/// Частицы одного оттенка уходят в один вызов отрисовки: пояс из тысяч
/// объектов стоит несколько вызовов, а не тысяч.
pub fn draw_belt(
    d: &mut Draw,
    belt: &Belt,
    elapsed: f32,
    center: Vec2,
    zoom: f32,
) -> Result<(), frglib::Error> {
    let view_rect = d.camera_rect();

    // На мелком зуме частица меньше пикселя и пропадает. Держим нижнюю
    // границу в экранных пикселях, а не в мировых единицах.
    let min_size = 1.3 / zoom.max(0.001);

    let mut buckets: [Vec<(Vec2, f32)>; SHADES as usize] = std::array::from_fn(|_| Vec::new());

    for particle in &belt.particles {
        let angle = particle.start_angle + elapsed * particle.speed;

        let position = center
            + Vec2::new(
                angle.cos() * particle.orbit_radius,
                angle.sin() * particle.orbit_radius * ORBIT_TILT,
            );

        if view_rect.contains_vec(position) {
            let shade = particle.shade.min(SHADES - 1) as usize;
            buckets[shade].push((position, particle.size.max(min_size)));
        }
    }

    for (shade, quads) in buckets.iter().enumerate() {
        if quads.is_empty() {
            continue;
        }

        let brightness = SHADE_BRIGHTNESS[shade];
        let base = belt.base_color;
        let color = Color::rgb(
            (base.r as f32 * brightness) as u8,
            (base.g as f32 * brightness) as u8,
            (base.b as f32 * brightness) as u8,
        );

        fill_quads(d, quads, color)?;
    }

    Ok(())
}
