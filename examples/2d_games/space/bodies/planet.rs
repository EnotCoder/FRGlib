use frglib::{Color, Image, Mat3, Vec2};

use super::moon::{get_moon, Moon};

pub struct Planet {
    pub name: &'static str,
    pub size: f32,
    pub color: Color,
    /// Спрайт планеты. `None` -- рисуем цветным кругом.
    pub texture: Option<Image>,
    pub orbit_radius: f32,
    pub speed: f32,
    pub start_angle: f32,
    pub moons: Vec<Moon>,
}

impl Planet {
    /// Где планета находится в данный момент.
    pub fn position(&self, elapsed: f32, center: Vec2) -> Vec2 {
        let angle = self.start_angle + elapsed * self.speed;

        (Mat3::translation(center)
            * Mat3::rotation(angle)
            * Mat3::translation(Vec2::new(self.orbit_radius, 0.0)))
        .transform_point(Vec2::ZERO)
    }

    /// Радиус планеты в мировых единицах.
    pub fn radius(&self) -> f32 {
        self.size * 0.5
    }
}

pub fn get_planets(
    mercury_texture: Image,
    vinera_image: Image,
    earth_image: Image,
    mars_image: Image,

    jupiter_image: Image,
    saturn_image: Image,
    uran_image: Image,
    neptune_image: Image,

    moon_textures: &[Option<Image>],
) -> Vec<Planet> {
    vec![
        // Меркурий.
        Planet {
            name: "Mercury",
            size: 8.0,
            color: Color::rgb(160, 150, 140),
            texture: Some(mercury_texture),
            orbit_radius: 100.0,
            speed: 1.5,
            start_angle: 0.0,
            moons: get_moon(1, moon_textures),
        },
        // Венера.
        Planet {
            name: "Venus",
            size: 14.0,
            color: Color::rgb(230, 190, 110),
            texture: Some(vinera_image),
            orbit_radius: 150.0,
            speed: 1.1,
            start_angle: 0.8,
            moons: get_moon(2, moon_textures),
        },
        // Земля.
        Planet {
            name: "Earth",
            size: 16.0,
            color: Color::rgb(70, 140, 255),
            texture: Some(earth_image),
            orbit_radius: 200.0,
            speed: 0.8,
            start_angle: 1.6,
            moons: get_moon(3, moon_textures),
        },
        // Марс.
        Planet {
            name: "Mars",
            size: 12.0,
            color: Color::rgb(220, 90, 60),
            texture: Some(mars_image),
            orbit_radius: 250.0,
            speed: 0.65,
            start_angle: 2.4,
            moons: get_moon(4, moon_textures),
        },
        // Юпитер.
        Planet {
            name: "Jupiter",
            size: 70.0,
            color: Color::rgb(210, 160, 120),
            texture: Some(jupiter_image),
            orbit_radius: 500.0,
            speed: 0.45,
            start_angle: 3.2,
            moons: get_moon(5, moon_textures),
        },
        // Сатурн.
        Planet {
            name: "Saturn",
            size: 60.0,
            color: Color::rgb(230, 205, 140),
            texture: Some(saturn_image),
            orbit_radius: 650.0,
            speed: 0.32,
            start_angle: 4.0,
            moons: get_moon(6, moon_textures),
        },
        // Уран.
        Planet {
            name: "Uranus",
            size: 38.0,
            color: Color::rgb(140, 220, 230),
            texture: Some(uran_image),
            orbit_radius: 740.0,
            speed: 0.23,
            start_angle: 4.8,
            moons: get_moon(7, moon_textures),
        },
        // Нептун.
        Planet {
            name: "Neptune",
            size: 40.0,
            color: Color::rgb(60, 90, 230),
            texture: Some(neptune_image),
            orbit_radius: 830.0,
            speed: 0.18,
            start_angle: 5.6,
            moons: get_moon(8, moon_textures),
        },
    ]
}
