use frglib::{Color, Image, Vec2};

pub struct Moon {
    pub name: &'static str,
    pub size: f32,
    pub color: Color,
    pub texture: Option<Image>,
    pub orbit_radius: f32,
    pub speed: f32,
    pub start_angle: f32,
}

impl Moon {
    /// Где спутник находится в данный момент вокруг своей планеты.
    pub fn position(&self, planet_position: Vec2, elapsed: f32) -> Vec2 {
        let angle = self.start_angle + elapsed * self.speed;

        planet_position
            + Vec2::new(
                angle.cos() * self.orbit_radius,
                angle.sin() * self.orbit_radius,
            )
    }

    /// Радиус спутника в мировых единицах.
    pub fn radius(&self) -> f32 {
        self.size * 0.5
    }
}

pub fn get_moon(planet_number: u32, moon_textures: &[Option<Image>]) -> Vec<Moon> {
    let get_tex = |idx: usize| moon_textures.get(idx).and_then(|opt| opt.clone());

    match planet_number {
        1 => vec![],
        2 => vec![],
        3 => vec![Moon {
            name: "Moon",
            size: 6.0,
            color: Color::rgb(210, 210, 210),
            texture: get_tex(0), // moon.png
            orbit_radius: 18.0,
            speed: 2.5,
            start_angle: 0.0,
        }],
        4 => vec![
            Moon {
                name: "Phobos",
                size: 4.0,
                color: Color::rgb(150, 140, 130),
                texture: get_tex(1), // thomas_deimos.png
                orbit_radius: 13.0,
                speed: 3.0,
                start_angle: 0.0,
            },
            Moon {
                name: "Deimos",
                size: 3.0,
                color: Color::rgb(190, 170, 150),
                texture: get_tex(1),
                orbit_radius: 21.0,
                speed: 1.8,
                start_angle: 2.0,
            },
        ],
        5 => vec![
            Moon {
                name: "Io",
                size: 6.0,
                color: Color::rgb(240, 210, 100),
                texture: get_tex(2), // io.png
                orbit_radius: 45.0,
                speed: 2.5,
                start_angle: 0.0,
            },
            Moon {
                name: "Callisto",
                size: 5.0,
                color: Color::rgb(200, 220, 240),
                texture: get_tex(3), // callisto.png
                orbit_radius: 60.0,
                speed: 1.7,
                start_angle: 2.0,
            },
            Moon {
                name: "Ganymede",
                size: 7.0,
                color: Color::rgb(160, 150, 140),
                texture: get_tex(4), // ganymede.png
                orbit_radius: 75.0,
                speed: 1.2,
                start_angle: 4.0,
            },
        ],
        6 => vec![
            Moon {
                name: "Titan",
                size: 7.0,
                color: Color::rgb(220, 170, 90),
                texture: get_tex(5), // titan.png
                orbit_radius: 35.0,
                speed: 1.8,
                start_angle: 1.0,
            },
            Moon {
                name: "Rhea",
                size: 4.0,
                color: Color::rgb(220, 230, 240),
                texture: get_tex(6), // rhea.png
                orbit_radius: 48.0,
                speed: 1.1,
                start_angle: 3.0,
            },
        ],
        7 => vec![Moon {
            name: "Titania",
            size: 5.0,
            color: Color::rgb(180, 190, 200),
            texture: get_tex(7), // titania.png
            orbit_radius: 35.0,
            speed: 1.7,
            start_angle: 0.0,
        }],
        8 => vec![Moon {
            name: "Triton",
            size: 6.0,
            color: Color::rgb(190, 210, 220),
            texture: get_tex(8), // triton.png
            orbit_radius: 35.0,
            speed: -1.5,
            start_angle: 0.0,
        }],
        _ => vec![],
    }
}
