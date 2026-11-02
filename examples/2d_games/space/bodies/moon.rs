use frglib::{Color, Image};

pub struct Moon {
    pub size: f32,
    pub color: Color,
    pub texture: Option<Image>,
    pub orbit_radius: f32,
    pub speed: f32,
    pub start_angle: f32,
}

pub fn get_moon(planet_number: u32, moon_textures: &[Option<Image>]) -> Vec<Moon> {
    let get_tex = |idx: usize| moon_textures.get(idx).and_then(|opt| opt.clone());

    match planet_number {
        1 => vec![],
        2 => vec![],
        3 => vec![Moon {
            size: 6.0,
            color: Color::rgb(210, 210, 210),
            texture: get_tex(0), // moon.png
            orbit_radius: 18.0,
            speed: 2.5,
            start_angle: 0.0,
        }],
        4 => vec![
            Moon {
                size: 4.0,
                color: Color::rgb(150, 140, 130),
                texture: get_tex(1), // thomas_deimos.png
                orbit_radius: 13.0,
                speed: 3.0,
                start_angle: 0.0,
            },
            Moon {
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
                size: 6.0,
                color: Color::rgb(240, 210, 100),
                texture: get_tex(2), // io.png
                orbit_radius: 28.0,
                speed: 2.5,
                start_angle: 0.0,
            },
            Moon {
                size: 5.0,
                color: Color::rgb(200, 220, 240),
                texture: get_tex(3), // callisto.png
                orbit_radius: 40.0,
                speed: 1.7,
                start_angle: 2.0,
            },
            Moon {
                size: 7.0,
                color: Color::rgb(160, 150, 140),
                texture: get_tex(4), // ganymede.png
                orbit_radius: 53.0,
                speed: 1.2,
                start_angle: 4.0,
            },
        ],
        6 => vec![
            Moon {
                size: 7.0,
                color: Color::rgb(220, 170, 90),
                texture: get_tex(5), // titan.png
                orbit_radius: 28.0,
                speed: 1.8,
                start_angle: 1.0,
            },
            Moon {
                size: 4.0,
                color: Color::rgb(220, 230, 240),
                texture: get_tex(6), // rhea.png
                orbit_radius: 40.0,
                speed: 1.1,
                start_angle: 3.0,
            },
        ],
        7 => vec![Moon {
            size: 5.0,
            color: Color::rgb(180, 190, 200),
            texture: get_tex(7), // titania.png
            orbit_radius: 25.0,
            speed: 1.7,
            start_angle: 0.0,
        }],
        8 => vec![Moon {
            size: 6.0,
            color: Color::rgb(190, 210, 220),
            texture: get_tex(8), // triton.png
            orbit_radius: 25.0,
            speed: -1.5,
            start_angle: 0.0,
        }],
        _ => vec![],
    }
}
