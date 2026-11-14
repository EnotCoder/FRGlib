use frglib::Vec2;

use super::Planet;

/// Положения всех тел на один кадр.
///
/// Позиции нужны и отрисовке, и выбору мышью, и панели, и камере. Раньше
/// каждое место считало их заново -- по три-четыре раза на кадр. Теперь
/// считается один раз здесь, а остальные только читают.
pub struct PlanetPose {
    pub position: Vec2,
    pub moons: Vec<Vec2>,
}

pub struct SystemPose {
    pub planets: Vec<PlanetPose>,
}

impl SystemPose {
    pub fn new(planets: &[Planet], elapsed: f32, center: Vec2) -> Self {
        let planets = planets
            .iter()
            .map(|planet| {
                let position = planet.position(elapsed, center);
                let moons = planet
                    .moons
                    .iter()
                    .map(|moon| moon.position(position, elapsed))
                    .collect();

                PlanetPose { position, moons }
            })
            .collect();

        Self { planets }
    }
}
