use frglib::{Draw, Object, Vec2};

use crate::bodies::*;

/// Рисует частицы пояса, вращающиеся вокруг Солнца.
pub fn draw_belt(
    d: &mut Draw,
    belt: &Belt,
    elapsed: f32,
    center: Vec2,
) -> Result<(), Box<dyn std::error::Error>> {
    let view_rect = d.camera_rect();
    for particle in &belt.particles {
        let angle = particle.start_angle + elapsed * particle.speed;

        let position = Vec2::new(
            center.x + angle.cos() * particle.orbit_radius,
            center.y + angle.sin() * particle.orbit_radius,
        );

        if view_rect.contains_vec(position) {
            let object = Object::new(position, Vec2::new(particle.size, particle.size));

            d.fill_object(&object, particle.color)?;
        }
    }

    Ok(())
}

/// Рисует частицы пояса, вращающиеся вокруг Солнца.
pub fn draw_cloud(
    d: &mut Draw,
    cloud: &Belt,
    elapsed: f32,
    center: Vec2,
) -> Result<(), Box<dyn std::error::Error>> {
    let view_rect = d.camera_rect();
    for particle in &cloud.particles {
        let angle = particle.start_angle + elapsed * particle.speed;

        let position = Vec2::new(
            center.x + angle.cos() * particle.orbit_radius,
            center.y + angle.sin() * particle.orbit_radius,
        );

        if view_rect.contains_vec(position) {
            let object = Object::new(position, Vec2::new(particle.size, particle.size));

            d.fill_object(&object, particle.color)?;
        }
    }

    Ok(())
}
