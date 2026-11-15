//! Подписи тел: собираются в мировых координатах, рисуются в экранных.

use frglib::{Anchor, Camera, Draw, Label, Vec2};

use super::Ui;
use crate::bodies::{Planet, PlanetPose};
use crate::consts::{
    LABEL_CULL_X, LABEL_CULL_Y, LABEL_GAP, LABEL_SPAWN_MARGIN, MOON_LABEL_ZOOM, PLANET_LABEL_ZOOM,
    SUN_SIZE,
};
use crate::view::world_to_screen;

/// Подпись, которую нужно нарисовать над телом.
pub struct ScreenLabel {
    name: &'static str,
    position: Vec2,
    radius: f32,
}

/// Собирает подписи только для тел в кадре и только на подходящем зуме.
#[allow(clippy::too_many_arguments)]
pub fn collect_name_labels(
    planets: &[Planet],
    poses: &[PlanetPose],
    center: Vec2,
    camera: Camera,
    window: Vec2,
    show_labels: bool,
    show_moons: bool,
) -> Vec<ScreenLabel> {
    if !show_labels {
        return Vec::new();
    }

    let mut labels = Vec::new();
    let mut push = |name: &'static str, position: Vec2, radius: f32| {
        let screen = world_to_screen(camera, position, window);

        if screen.x > -LABEL_SPAWN_MARGIN
            && screen.x < window.x + LABEL_SPAWN_MARGIN
            && screen.y > -LABEL_SPAWN_MARGIN
            && screen.y < window.y + LABEL_SPAWN_MARGIN
        {
            labels.push(ScreenLabel {
                name,
                position,
                radius,
            });
        }
    };

    push("Sun", center, SUN_SIZE * 0.5);

    // На общем плане подписи планет, спутники -- только при приближении.
    if camera.zoom > PLANET_LABEL_ZOOM {
        for (planet, pose) in planets.iter().zip(poses) {
            push(planet.name, pose.position, planet.radius());
        }
    }

    if show_moons && camera.zoom > MOON_LABEL_ZOOM {
        for (planet, pose) in planets.iter().zip(poses) {
            for (moon, moon_position) in planet.moons.iter().zip(&pose.moons) {
                push(moon.name, *moon_position, moon.radius());
            }
        }
    }

    labels
}

impl Ui<'_> {
    /// Рисует подписи над телами, отсекая те, что ушли за край окна.
    pub fn draw_labels(
        &self,
        d: &mut Draw,
        labels: &[ScreenLabel],
        camera: Camera,
        window: Vec2,
    ) -> Result<(), frglib::Error> {
        for label in labels {
            let position = world_to_screen(camera, label.position, window);
            let y = position.y - label.radius * camera.zoom - LABEL_GAP;

            if position.x < -LABEL_CULL_X
                || position.x > window.x + LABEL_CULL_X
                || y < -LABEL_CULL_Y
                || y > window.y + LABEL_CULL_Y
            {
                continue;
            }

            Label::new(self.hud.get(label.name))
                .at(position.x, y)
                .anchored(Anchor::Bottom)
                .draw(d)?;
        }

        Ok(())
    }
}
