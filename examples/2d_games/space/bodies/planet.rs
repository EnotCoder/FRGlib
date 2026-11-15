use frglib::{Color, Image, Vec2};

use crate::consts::ORBIT_TILT;

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

    /// Настоящий радиус тела, км. Для карточки в интерфейсе.
    pub radius_km: f32,
    /// Среднее расстояние от Солнца, км.
    pub distance_km: f32,
    /// Сидерический период обращения, земные сутки.
    pub year_days: f32,
    /// Сколько спутников известно (не только тех, что нарисованы).
    pub moon_count: u32,
}

impl Planet {
    /// Где планета находится в данный момент.
    ///
    /// `y` сжат на [`ORBIT_TILT`], поэтому орбита -- эллипс, а не круг.
    pub fn position(&self, elapsed: f32, center: Vec2) -> Vec2 {
        let angle = self.start_angle + elapsed * self.speed;

        center
            + Vec2::new(
                angle.cos() * self.orbit_radius,
                angle.sin() * self.orbit_radius * ORBIT_TILT,
            )
    }

    /// Радиус планеты в мировых единицах.
    pub fn radius(&self) -> f32 {
        self.size * 0.5
    }
}

#[allow(clippy::too_many_arguments)]
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
            radius_km: 2_440.0,
            distance_km: 57_900_000.0,
            year_days: 88.0,
            moon_count: 0,
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
            radius_km: 6_052.0,
            distance_km: 108_200_000.0,
            year_days: 224.7,
            moon_count: 0,
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
            radius_km: 6_371.0,
            distance_km: 149_600_000.0,
            year_days: 365.3,
            moon_count: 1,
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
            radius_km: 3_390.0,
            distance_km: 227_900_000.0,
            year_days: 687.0,
            moon_count: 2,
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
            radius_km: 69_911.0,
            distance_km: 778_500_000.0,
            year_days: 4_331.0,
            moon_count: 95,
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
            radius_km: 58_232.0,
            distance_km: 1_434_000_000.0,
            year_days: 10_747.0,
            moon_count: 146,
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
            radius_km: 25_362.0,
            distance_km: 2_871_000_000.0,
            year_days: 30_589.0,
            moon_count: 28,
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
            radius_km: 24_622.0,
            distance_km: 4_495_000_000.0,
            year_days: 59_800.0,
            moon_count: 16,
        },
    ]
}
