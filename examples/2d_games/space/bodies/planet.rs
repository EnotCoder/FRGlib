use frglib::Color;

use super::moon::{Moon, get_moon};

pub struct Planet {
    pub size: f32,
    pub color: Color,
    pub orbit_radius: f32,
    pub speed: f32,
    pub start_angle: f32,
    pub moons: Vec<Moon>,
}


pub fn get_planets() -> Vec<Planet>{
    vec![
        // Меркурий.
        Planet {
            size: 8.0,
            color: Color::rgb(160, 150, 140),
            orbit_radius: 45.0,
            speed: 1.5,
            start_angle: 0.0,
            moons: get_moon(1),
        },

        // Венера.
        Planet {
            size: 14.0,
            color: Color::rgb(230, 190, 110),
            orbit_radius: 80.0,
            speed: 1.1,
            start_angle: 0.8,
            moons: get_moon(2),
        },

        // Земля.
        Planet {
            size: 16.0,
            color: Color::rgb(70, 140, 255),
            orbit_radius: 120.0,
            speed: 0.8,
            start_angle: 1.6,
            moons: get_moon(3),
        },

        // Марс.
        Planet {
            size: 12.0,
            color: Color::rgb(220, 90, 60),
            orbit_radius: 165.0,
            speed: 0.65,
            start_angle: 2.4,
            moons: get_moon(4),
        },

        // Юпитер.
        Planet {
            size: 34.0,
            color: Color::rgb(210, 160, 120),
            orbit_radius: 220.0,
            speed: 0.45,
            start_angle: 3.2,
            moons: get_moon(5),
        },

        // Сатурн.
        Planet {
            size: 28.0,
            color: Color::rgb(230, 205, 140),
            orbit_radius: 285.0,
            speed: 0.32,
            start_angle: 4.0,
            moons: get_moon(6),
        },

        // Уран.
        Planet {
            size: 22.0,
            color: Color::rgb(140, 220, 230),
            orbit_radius: 345.0,
            speed: 0.23,
            start_angle: 4.8,
            moons: get_moon(7),
        },

        // Нептун.
        Planet {
            size: 22.0,
            color: Color::rgb(60, 90, 230),
            orbit_radius: 400.0,
            speed: 0.18,
            start_angle: 5.6,
            moons: get_moon(8),
        },
    ]
}