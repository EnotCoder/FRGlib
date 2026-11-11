use frglib::{Color, Object, Vec2};
use std::f32::consts::TAU;

use crate::bodies::*;

/// Количество сегментов в круге свечения.
const GLOW_SEGMENTS: usize = 48;

/// Рисует мягкое свечение вокруг тела: несколько вложенных кругов.
///
/// Свечение рисуется до самого тела, поэтому виден только ореол по краю.
pub fn draw_glow(
    d: &mut frglib::Draw,
    center: Vec2,
    radius: f32,
    zoom: f32,
    color: Color,
) -> Result<(), frglib::Error> {
    let margin = 4.0 / zoom.max(0.001);
    let mut indices = Vec::with_capacity(GLOW_SEGMENTS * 3);

    for i in 0..GLOW_SEGMENTS {
        let next = (i + 1) % GLOW_SEGMENTS;
        indices.extend_from_slice(&[0, 1 + i as i32, 1 + next as i32]);
    }

    // Внутренний круг самый яркий, внешние -- всё прозрачнее.
    for (layer, factor) in [1.0f32, 1.45, 1.9].into_iter().enumerate() {
        let glow_radius = radius + margin * factor;
        let mut points = Vec::with_capacity(GLOW_SEGMENTS + 1);
        points.push((center.x, center.y));

        for i in 0..GLOW_SEGMENTS {
            let angle = TAU * i as f32 / GLOW_SEGMENTS as f32;
            points.push((
                center.x + angle.cos() * glow_radius,
                center.y + angle.sin() * glow_radius,
            ));
        }

        let alpha = color.a as f32 / (1.0 + layer as f32 * 1.2);
        let layer_color = Color::rgba(color.r, color.g, color.b, alpha as u8);

        d.fill_object_polygon(
            &Object::new(center, Vec2::ZERO),
            &points,
            &indices,
            layer_color,
        )?;
    }

    Ok(())
}

/// Рисует тонкое кольцо вокруг тела.
pub fn draw_ring(
    d: &mut frglib::Draw,
    center: Vec2,
    radius: f32,
    zoom: f32,
    color: Color,
) -> Result<(), frglib::Error> {
    let ring_radius = radius + 3.0 / zoom.max(0.001);
    let points = (0..GLOW_SEGMENTS)
        .map(|i| {
            let angle = TAU * i as f32 / GLOW_SEGMENTS as f32;
            (
                center.x + angle.cos() * ring_radius,
                center.y + angle.sin() * ring_radius,
            )
        })
        .collect::<Vec<_>>();

    d.draw_object_polygon(&Object::new(center, Vec2::ZERO), &points, color)
}

pub fn draw_bodies(
    d: &mut frglib::Draw,
    planets: &[Planet],
    elapsed: f32,
    center: Vec2,
    orbit_thickness: f32,
    orbit_color: Color,
) -> Result<(), Box<dyn std::error::Error>> {
    let view_rect = d.camera_rect();

    for planet in planets {
        let planet_pos = planet.position(elapsed, center);

        // Орбиты спутников
        for moon in &planet.moons {
            // Адаптивное количество сегментов: увеличиваем плотность для чёткости
            let segments = (moon.orbit_radius * 4.0).max(128.0) as i32;

            for i in 0..segments {
                let angle = TAU * i as f32 / segments as f32;
                let point = Vec2::new(
                    planet_pos.x + angle.cos() * moon.orbit_radius,
                    planet_pos.y + angle.sin() * moon.orbit_radius,
                );

                if view_rect.contains_vec(point) {
                    let dot = Object::new(point, Vec2::new(orbit_thickness, orbit_thickness));
                    d.fill_object(&dot, orbit_color)?;
                }
            }
        }

        // Планета.
        let planet_obj = Object::new(planet_pos, Vec2::new(planet.size, planet.size))
            .with_rotation(elapsed * planet.speed * 80.0);

        if view_rect.intersects(planet_obj.rect()) {
            match &planet.texture {
                Some(image) => d.fill_textured_object(&planet_obj, image)?,
                None => d.fill_object(&planet_obj, planet.color)?,
            }
        }

        // Спутники.
        for moon in &planet.moons {
            let moon_pos = moon.position(planet_pos, elapsed);
            let moon_obj = Object::new(moon_pos, Vec2::new(moon.size, moon.size));

            if view_rect.intersects(moon_obj.rect()) {
                match &moon.texture {
                    Some(image) => d.fill_textured_object(&moon_obj, image)?,
                    None => d.fill_object(&moon_obj, moon.color)?,
                }
            }
        }
    }

    Ok(())
}

pub fn draw_orbit(
    d: &mut frglib::Draw,
    planets: &[Planet],
    center: Vec2,
    orbit_thickness: f32,
    orbit_color: Color,
) -> Result<(), Box<dyn std::error::Error>> {
    let view_rect = d.camera_rect();

    for planet in planets {
        // Увеличиваем множитель с 2.0 до 8.0 для максимальной чёткости при любом зуме
        let segments = (planet.orbit_radius * 8.0).max(1440.0) as i32;

        for i in 0..segments {
            let angle = TAU * i as f32 / segments as f32;

            let point = Vec2::new(
                center.x + angle.cos() * planet.orbit_radius,
                center.y + angle.sin() * planet.orbit_radius,
            );

            // Оптимизация: рисуем только если точка в кадре
            if view_rect.contains_vec(point) {
                let dot = Object::new(point, Vec2::new(orbit_thickness, orbit_thickness));

                d.fill_object(&dot, orbit_color)?;
            }
        }
    }

    Ok(())
}
