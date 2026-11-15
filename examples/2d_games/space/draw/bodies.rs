//! Сцена тел, отсортированная по глубине.
//!
//! Чем меньше `y`, тем дальше тело и тем раньше оно рисуется. Порядок даёт
//! окклюзию бесплатно: Солнце закрывает планеты на дальней стороне орбиты,
//! а планета -- спутники, ушедшие за её диск.

use frglib::{Draw, Image, Vec2};

use crate::bodies::{Planet, PlanetPose};

use super::moon::draw_moon;
use super::planet::draw_planet;
use super::sun::draw_sun_body;

/// Элемент сцены в порядке отрисовки.
#[derive(Clone, Copy)]
enum SceneItem {
    Sun,
    Planet(usize),
    Moon(usize, usize),
}

/// Рисует все тела одной сценой, отсортированной по глубине.
///
/// Свечение и обводка каждого тела рисуются в том же проходе, что и само
/// тело. Поэтому близкий объект перекрывает и диск, и ореол, и кольцо
/// дальнего.
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
