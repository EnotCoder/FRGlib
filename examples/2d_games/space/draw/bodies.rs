use frglib::{Color, Mat3, Object, Vec2};
use std::f32::consts::TAU;

use crate::bodies::*;

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
        let angle = planet.start_angle + elapsed * planet.speed;

        let planet_transform = Mat3::translation(center)
            * Mat3::rotation(angle)
            * Mat3::translation(Vec2::new(planet.orbit_radius, 0.0));

        let planet_pos = planet_transform.transform_point(Vec2::ZERO);

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
            let moon_angle = moon.start_angle + elapsed * moon.speed;
            let moon_transform = planet_transform
                * Mat3::rotation(moon_angle)
                * Mat3::translation(Vec2::new(moon.orbit_radius, 0.0));

            let moon_pos = moon_transform.transform_point(Vec2::ZERO);
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
