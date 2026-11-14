use frglib::{Color, Draw, Image, Object, Rect, Vec2};
use std::f32::consts::{PI, TAU};

use crate::bodies::{Moon, Planet, PlanetPose, ORBIT_TILT};

use super::poly::{fill_disc, fill_ellipse_ring_half, fill_night_side, stroke_ellipse_arc};
use super::sun::draw_sun;

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
fn draw_glow(
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

/// Цвет свечения тела: обычное, выбранное или подсвеченное курсором.
fn glow_color(name: &str, hovered: Option<&str>, selected: Option<&str>, elapsed: f32) -> Color {
    if hovered == Some(name) {
        let pulse = (elapsed * 6.0).sin() * 0.5 + 0.5;

        Color::rgba(185, 225, 255, (150.0 + pulse * 70.0) as u8)
    } else if selected == Some(name) {
        Color::rgba(155, 210, 255, 110)
    } else {
        Color::rgba(130, 175, 255, 40)
    }
}

/// Элемент сцены. Все тела сортируются по `y`: чем меньше `y`, тем дальше
/// тело и тем раньше оно рисуется. Поэтому Солнце закрывает планеты на
/// дальней стороне орбиты, а планета -- спутники, ушедшие за её диск.
#[derive(Clone, Copy)]
enum SceneItem {
    Sun,
    Planet(usize),
    Moon(usize, usize),
}

/// Обводка выбранного и подсвеченного тела. Рисуется сразу после самого
/// тела, поэтому более близкие объекты естественно перекрывают её.
#[allow(clippy::too_many_arguments)]
fn draw_marker_rings(
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
        draw_ring(d, position, radius, zoom, Color::rgba(155, 210, 255, 170))?;
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

#[allow(clippy::too_many_arguments)]
fn draw_sun_body(
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
    draw_sun(d, center, radius, elapsed)?;
    fill_disc(d, center, radius * 0.99, Color::rgba(255, 224, 150, 255))?;

    let object = Object::new(center, Vec2::new(sun_size, sun_size)).with_rotation(elapsed * 20.0);

    d.fill_textured_object(&object, sun_image)?;

    draw_marker_rings(d, "Sun", center, radius, zoom, hovered, selected, elapsed)
}

#[allow(clippy::too_many_arguments)]
fn draw_planet(
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

#[allow(clippy::too_many_arguments)]
fn draw_moon(
    d: &mut Draw,
    moon: &Moon,
    position: Vec2,
    sun: Vec2,
    zoom: f32,
    view_rect: Rect,
    hovered: Option<&str>,
    selected: Option<&str>,
    elapsed: f32,
) -> Result<(), frglib::Error> {
    let radius = moon.radius();

    if !body_visible(view_rect, position, radius) {
        return Ok(());
    }

    draw_glow(
        d,
        position,
        radius,
        zoom,
        glow_color(moon.name, hovered, selected, elapsed),
    )?;

    let moon_object = Object::new(position, Vec2::new(moon.size, moon.size));

    match &moon.texture {
        Some(image) => d.fill_textured_object(&moon_object, image)?,
        None => d.fill_object(&moon_object, moon.color)?,
    }

    draw_night_side(d, sun, position, radius)?;

    draw_marker_rings(
        d, moon.name, position, radius, zoom, hovered, selected, elapsed,
    )
}

/// Рисует все тела одной сценой, отсортированной по глубине.
///
/// Свечение и обводка каждого тела рисуются в том же проходе, что и само
/// тело. Поэтому близкий объект перекрывает и диск, и ореол, и кольцо
/// дальнего -- именно это даёт окклюзию без отдельных проходов.
#[allow(clippy::too_many_arguments)]
pub fn draw_bodies(
    d: &mut Draw,
    planets: &[Planet],
    poses: &[PlanetPose],
    elapsed: f32,
    center: Vec2,
    zoom: f32,
    sun_size: f32,
    sun_image: &Image,
    show_moons: bool,
    hovered: Option<&str>,
    selected: Option<&str>,
) -> Result<(), frglib::Error> {
    let view_rect = d.camera_rect();

    // Солнце в списке первым, планеты -- раньше своих спутников. Стабильная
    // сортировка сохраняет этот порядок при равной глубине, поэтому спутник
    // сбоку от планеты остаётся поверх её диска.
    let mut scene = Vec::with_capacity(1 + planets.len() * 2);
    scene.push(SceneItem::Sun);

    for (planet_index, planet) in planets.iter().enumerate() {
        scene.push(SceneItem::Planet(planet_index));

        if show_moons {
            for moon_index in 0..planet.moons.len() {
                scene.push(SceneItem::Moon(planet_index, moon_index));
            }
        }
    }

    let depth = |item: SceneItem| -> f32 {
        match item {
            SceneItem::Sun => center.y,
            SceneItem::Planet(index) => poses[index].position.y,
            SceneItem::Moon(planet, moon) => poses[planet].moons[moon].y,
        }
    };

    scene.sort_by(|a, b| {
        depth(*a)
            .partial_cmp(&depth(*b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for item in scene {
        match item {
            SceneItem::Sun => draw_sun_body(
                d, center, sun_size, elapsed, zoom, sun_image, hovered, selected, view_rect,
            )?,
            SceneItem::Planet(index) => draw_planet(
                d,
                &planets[index],
                &poses[index],
                elapsed,
                center,
                zoom,
                view_rect,
                hovered,
                selected,
            )?,
            SceneItem::Moon(planet_index, moon_index) => draw_moon(
                d,
                &planets[planet_index].moons[moon_index],
                poses[planet_index].moons[moon_index],
                center,
                zoom,
                view_rect,
                hovered,
                selected,
                elapsed,
            )?,
        }
    }

    Ok(())
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
