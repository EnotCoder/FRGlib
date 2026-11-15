//! Орбиты планет и спутников.

use std::f32::consts::{PI, TAU};

use frglib::{Color, Draw, Vec2};

use crate::bodies::{Planet, PlanetPose};
use crate::consts::ORBIT_TILT;

use super::cull::orbit_visible;
use super::poly::stroke_ellipse_arc;

/// Цвета орбит: дальняя половина тусклее, ближняя ярче -- так эллипс
/// читается как наклонённый круг, а не как плоская линия.
const ORBIT_FAR: Color = Color::rgb(36, 40, 55);
const ORBIT_NEAR: Color = Color::rgb(76, 84, 104);

const MOON_ORBIT_FAR: Color = Color::rgb(40, 45, 58);
const MOON_ORBIT_NEAR: Color = Color::rgb(62, 70, 86);

/// Орбиты планет и спутников. Дальняя половина каждого эллипса тусклее.
pub fn draw_orbits(
    d: &mut Draw,
    planets: &[Planet],
    poses: &[PlanetPose],
    center: Vec2,
    show_moons: bool,
    zoom: f32,
) -> Result<(), frglib::Error> {
    let view_rect = d.camera_rect();
    let width = 1.3 / zoom.max(0.001);

    for (planet, pose) in planets.iter().zip(poses) {
        if !orbit_visible(view_rect, center, planet.orbit_radius) {
            continue;
        }

        let segments = (planet.orbit_radius * 0.35).clamp(96.0, 256.0) as usize;

        stroke_ellipse_arc(
            d,
            center,
            planet.orbit_radius,
            ORBIT_TILT,
            PI,
            TAU,
            segments,
            width,
            ORBIT_FAR,
        )?;
        stroke_ellipse_arc(
            d,
            center,
            planet.orbit_radius,
            ORBIT_TILT,
            0.0,
            PI,
            segments,
            width,
            ORBIT_NEAR,
        )?;

        if !show_moons {
            continue;
        }

        for moon in &planet.moons {
            if !orbit_visible(view_rect, pose.position, moon.orbit_radius) {
                continue;
            }

            let segments = (moon.orbit_radius * 3.0).clamp(48.0, 128.0) as usize;

            stroke_ellipse_arc(
                d,
                pose.position,
                moon.orbit_radius,
                ORBIT_TILT,
                PI,
                TAU,
                segments,
                width,
                MOON_ORBIT_FAR,
            )?;
            stroke_ellipse_arc(
                d,
                pose.position,
                moon.orbit_radius,
                ORBIT_TILT,
                0.0,
                PI,
                segments,
                width,
                MOON_ORBIT_NEAR,
            )?;
        }
    }

    Ok(())
}
