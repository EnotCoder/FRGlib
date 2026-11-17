//! Выбор тел мышью и поиск их текущих координат.

use frglib::Vec2;

use crate::bodies::{Planet, PlanetPose};
use crate::consts::SUN_SIZE;
use crate::ui::{moon_info, planet_info, sun_info, BodyInfo};

/// Текущее положение тела по имени. Выбранное тело движется вместе с орбитой.
pub fn body_position(
    planets: &[Planet],
    poses: &[PlanetPose],
    name: &str,
    center: Vec2,
) -> Option<Vec2> {
    if name == "Sun" {
        return Some(center);
    }

    for (planet, pose) in planets.iter().zip(poses) {
        if planet.name == name {
            return Some(pose.position);
        }

        for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
            if moon.name == name {
                return Some(*moon_position);
            }
        }
    }

    None
}

/// Ищет тело под курсором. Из нескольких попаданий выбирает то, чей край
/// ближе всего к точке клика.
///
/// `min_hit_radius` -- минимальный радиус попадания в пикселях экрана: мелкие
/// спутники иначе невозможно поймать ни мышью, ни пальцем.
pub fn pick_body(
    planets: &[Planet],
    poses: &[PlanetPose],
    world: Vec2,
    center: Vec2,
    zoom: f32,
    min_hit_radius: f32,
) -> Option<BodyInfo> {
    let mut best: Option<(f32, BodyInfo)> = None;
    let mut consider = |distance: f32, radius: f32, info: BodyInfo| {
        let hit_radius = radius.max(min_hit_radius / zoom.max(0.001));

        if distance <= hit_radius {
            let offset = distance / hit_radius;

            if best.is_none_or(|(best_offset, _)| offset < best_offset) {
                best = Some((offset, info));
            }
        }
    };

    for (planet, pose) in planets.iter().zip(poses) {
        for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
            consider(
                (*moon_position - world).length(),
                moon.radius(),
                moon_info(moon),
            );
        }

        consider(
            (pose.position - world).length(),
            planet.radius(),
            planet_info(planet),
        );
    }

    consider((center - world).length(), SUN_SIZE * 0.5, sun_info());

    best.map(|(_, info)| info)
}

/// Выбирает планету по номеру и переводит на неё камеру.
pub fn select_planet(
    planets: &[Planet],
    index: usize,
    selected: &mut Option<BodyInfo>,
    follow: &mut Option<&'static str>,
) {
    if let Some(planet) = planets.get(index) {
        *selected = Some(planet_info(planet));
        *follow = Some(planet.name);
    }
}
