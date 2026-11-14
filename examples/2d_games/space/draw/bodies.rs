use frglib::{Color, Draw, Object, Rect, Vec2};
use std::f32::consts::{PI, TAU};

use crate::bodies::{Moon, Planet, PlanetPose, ORBIT_TILT};

use super::poly::{fill_disc, fill_ellipse_ring_half, fill_night_side, stroke_ellipse_arc};

/// Цвета орбит: дальняя половина тусклее, ближняя ярче -- так эллипс
/// читается как наклонённый круг, а не как плоская линия.
const ORBIT_FAR: Color = Color::rgb(36, 40, 55);
const ORBIT_NEAR: Color = Color::rgb(76, 84, 104);

const MOON_ORBIT_FAR: Color = Color::rgb(40, 45, 58);
const MOON_ORBIT_NEAR: Color = Color::rgb(62, 70, 86);

/// Цвет ночной стороны: почти чёрный, с лёгкой синью.
const NIGHT: Color = Color::rgba(2, 4, 12, 112);

/// Считает, попадает ли эллипс орбиты в кадр.
fn orbit_visible(view_rect: Rect, center: Vec2, radius: f32) -> bool {
    let rect = Rect::new(
        center.x - radius,
        center.y - radius * ORBIT_TILT,
        radius * 2.0,
        radius * 2.0 * ORBIT_TILT,
    );

    view_rect.intersects(rect)
}

fn body_visible(view_rect: Rect, position: Vec2, radius: f32) -> bool {
    let rect = Rect::new(
        position.x - radius,
        position.y - radius,
        radius * 2.0,
        radius * 2.0,
    );

    view_rect.intersects(rect)
}

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

/// Рисует тонкое кольцо вокруг тела.
pub fn draw_ring(
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

/// Тела: сортировка по `y`, тень на ночной стороне, кольца Сатурна,
/// спутники вокруг своих планет.
pub fn draw_bodies(
    d: &mut Draw,
    planets: &[Planet],
    poses: &[PlanetPose],
    elapsed: f32,
    center: Vec2,
) -> Result<(), frglib::Error> {
    let view_rect = d.camera_rect();

    let mut order = (0..planets.len()).collect::<Vec<_>>();
    order.sort_by(|a, b| {
        poses[*a]
            .position
            .y
            .partial_cmp(&poses[*b].position.y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for index in order {
        let planet = &planets[index];
        let pose = &poses[index];

        if !body_visible(view_rect, pose.position, planet.radius()) {
            continue;
        }

        let is_saturn = planet.name == "Saturn";

        if is_saturn {
            draw_saturn_rings(d, pose.position, planet.radius(), false)?;
        }

        // Спутники на дальней стороне рисуются до планеты и ею закрываются.
        for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
            if moon_position.y < pose.position.y {
                draw_moon(d, moon, *moon_position, center, view_rect)?;
            }
        }

        let planet_object = Object::new(pose.position, Vec2::new(planet.size, planet.size))
            .with_rotation(elapsed * planet.speed * 80.0);

        match &planet.texture {
            Some(image) => d.fill_textured_object(&planet_object, image)?,
            None => d.fill_object(&planet_object, planet.color)?,
        }

        draw_night_side(d, center, pose.position, planet.radius())?;

        if is_saturn {
            draw_saturn_rings(d, pose.position, planet.radius(), true)?;
        }

        for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
            if moon_position.y >= pose.position.y {
                draw_moon(d, moon, *moon_position, center, view_rect)?;
            }
        }
    }

    Ok(())
}

fn draw_moon(
    d: &mut Draw,
    moon: &Moon,
    position: Vec2,
    sun: Vec2,
    view_rect: Rect,
) -> Result<(), frglib::Error> {
    if !body_visible(view_rect, position, moon.radius()) {
        return Ok(());
    }

    let moon_object = Object::new(position, Vec2::new(moon.size, moon.size));

    match &moon.texture {
        Some(image) => d.fill_textured_object(&moon_object, image)?,
        None => d.fill_object(&moon_object, moon.color)?,
    }

    draw_night_side(d, sun, position, moon.radius())
}

/// Затемняет половину диска, отвёрнутую от Солнца.
fn draw_night_side(d: &mut Draw, sun: Vec2, body: Vec2, radius: f32) -> Result<(), frglib::Error> {
    let direction = sun - body;

    if direction.length_squared() < f32::EPSILON {
        return Ok(());
    }

    let angle = direction.y.atan2(direction.x);

    fill_night_side(d, body, radius, angle, NIGHT)
}
