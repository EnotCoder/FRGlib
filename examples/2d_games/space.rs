use frglib::{
    Color, Event, FrameClock, Keycode, Mat3, Object, Vec2, Window,
};
use std::f32::consts::TAU;

struct Star {
    position: Vec2,
    size: f32,
    color: Color,
}

struct Moon {
    size: f32,
    color: Color,
    orbit_radius: f32,
    speed: f32,
    start_angle: f32,
}

struct Planet {
    size: f32,
    color: Color,
    orbit_radius: f32,
    speed: f32,
    start_angle: f32,
    moons: Vec<Moon>,
}

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGlib: planets and stars")
        .size(1200, 900)
        .background(Color::rgb(20, 20, 25))
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut elapsed = 0.0f32;

    let orbit_thickness = 2.0;
    let orbit_color = Color::rgb(80, 80, 85);

    // Настройки планет и спутников.
    let planets = vec![
        // Меркурий.
        Planet {
            size: 8.0,
            color: Color::rgb(160, 150, 140),
            orbit_radius: 45.0,
            speed: 1.5,
            start_angle: 0.0,
            moons: vec![],
        },

        // Венера.
        Planet {
            size: 14.0,
            color: Color::rgb(230, 190, 110),
            orbit_radius: 80.0,
            speed: 1.1,
            start_angle: 0.8,
            moons: vec![],
        },

        // Земля.
        Planet {
            size: 16.0,
            color: Color::rgb(70, 140, 255),
            orbit_radius: 120.0,
            speed: 0.8,
            start_angle: 1.6,
            moons: vec![
                Moon {
                    size: 6.0,
                    color: Color::rgb(210, 210, 210),
                    orbit_radius: 18.0,
                    speed: 2.5,
                    start_angle: 0.0,
                },
            ],
        },

        // Марс.
        Planet {
            size: 12.0,
            color: Color::rgb(220, 90, 60),
            orbit_radius: 165.0,
            speed: 0.65,
            start_angle: 2.4,
            moons: vec![
                Moon {
                    size: 4.0,
                    color: Color::rgb(150, 140, 130),
                    orbit_radius: 13.0,
                    speed: 3.0,
                    start_angle: 0.0,
                },
                Moon {
                    size: 3.0,
                    color: Color::rgb(190, 170, 150),
                    orbit_radius: 21.0,
                    speed: 1.8,
                    start_angle: 2.0,
                },
            ],
        },

        // Юпитер.
        Planet {
            size: 34.0,
            color: Color::rgb(210, 160, 120),
            orbit_radius: 220.0,
            speed: 0.45,
            start_angle: 3.2,
            moons: vec![
                Moon {
                    size: 6.0,
                    color: Color::rgb(240, 210, 100),
                    orbit_radius: 28.0,
                    speed: 2.5,
                    start_angle: 0.0,
                },
                Moon {
                    size: 5.0,
                    color: Color::rgb(200, 220, 240),
                    orbit_radius: 40.0,
                    speed: 1.7,
                    start_angle: 2.0,
                },
                Moon {
                    size: 7.0,
                    color: Color::rgb(160, 150, 140),
                    orbit_radius: 53.0,
                    speed: 1.2,
                    start_angle: 4.0,
                },
            ],
        },

        // Сатурн.
        Planet {
            size: 28.0,
            color: Color::rgb(230, 205, 140),
            orbit_radius: 285.0,
            speed: 0.32,
            start_angle: 4.0,
            moons: vec![
                Moon {
                    size: 7.0,
                    color: Color::rgb(220, 170, 90),
                    orbit_radius: 28.0,
                    speed: 1.8,
                    start_angle: 1.0,
                },
                Moon {
                    size: 4.0,
                    color: Color::rgb(220, 230, 240),
                    orbit_radius: 40.0,
                    speed: 1.1,
                    start_angle: 3.0,
                },
            ],
        },

        // Уран.
        Planet {
            size: 22.0,
            color: Color::rgb(140, 220, 230),
            orbit_radius: 345.0,
            speed: 0.23,
            start_angle: 4.8,
            moons: vec![
                Moon {
                    size: 5.0,
                    color: Color::rgb(180, 190, 200),
                    orbit_radius: 25.0,
                    speed: 1.7,
                    start_angle: 0.0,
                },
            ],
        },

        // Нептун.
        Planet {
            size: 22.0,
            color: Color::rgb(60, 90, 230),
            orbit_radius: 400.0,
            speed: 0.18,
            start_angle: 5.6,
            moons: vec![
                Moon {
                    size: 6.0,
                    color: Color::rgb(190, 210, 220),
                    orbit_radius: 25.0,
                    speed: -1.5,
                    start_angle: 0.0,
                },
            ],
        },
    ];

    // Создаём звёзды один раз.
    let mut stars = Vec::new();
    let mut seed: u32 = 123456789;

    let mut next_random = || {
        seed = seed
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);

        (seed >> 8) as f32 / 16_777_216.0
    };

    for _ in 0..250 {
        let x = next_random();
        let y = next_random();

        let size = 1.0 + next_random() * 1.5;
        let brightness = (90.0 + next_random() * 150.0) as u8;

        stars.push(Star {
            position: Vec2::new(x, y),
            size,
            color: Color::rgb(brightness, brightness, brightness),
        });
    }

    loop {
        let dt = clock.dt();
        elapsed += dt;

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),

                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),

                _ => {}
            }
        }

        let (w, h) = window.size();
        let window_size = Vec2::new(w as f32, h as f32);

        let center = Vec2::new(
            window_size.x / 2.0,
            window_size.y / 2.0,
        );

        window.frame_with(|d| {
            // Звёзды на заднем плане.
            for star in &stars {
                let position = Vec2::new(
                    star.position.x * window_size.x,
                    star.position.y * window_size.y,
                );

                let star_obj = Object::new(
                    position,
                    Vec2::new(star.size, star.size),
                );

                d.fill_object(&star_obj, star.color)?;
            }

            // Орбиты планет.
            for planet in &planets {
                let segments = 1440;

                for i in 0..segments {
                    let angle = TAU * i as f32 / segments as f32;

                    let point = Vec2::new(
                        center.x + angle.cos() * planet.orbit_radius,
                        center.y + angle.sin() * planet.orbit_radius,
                    );

                    let dot = Object::new(
                        point,
                        Vec2::new(orbit_thickness, orbit_thickness),
                    );

                    d.fill_object(&dot, orbit_color)?;
                }
            }

            // Солнце.
            let sun = Object::new(
                center,
                Vec2::new(35.0, 35.0),
            );

            d.fill_object(&sun, Color::rgb(255, 200, 50))?;

            for planet in &planets {
                let angle =
                    planet.start_angle + elapsed * planet.speed;

                let planet_transform =
                    Mat3::translation(center)
                    * Mat3::rotation(angle)
                    * Mat3::translation(Vec2::new(
                        planet.orbit_radius,
                        0.0,
                    ));

                let planet_pos =
                    planet_transform.transform_point(Vec2::ZERO);

                // Орбиты спутников вокруг текущей позиции планеты.
                for moon in &planet.moons {
                    let segments = 360;

                    for i in 0..segments {
                        let angle =
                            TAU * i as f32 / segments as f32;

                        let point = Vec2::new(
                            planet_pos.x
                                + angle.cos() * moon.orbit_radius,
                            planet_pos.y
                                + angle.sin() * moon.orbit_radius,
                        );

                        let dot = Object::new(
                            point,
                            Vec2::new(
                                orbit_thickness,
                                orbit_thickness,
                            ),
                        );

                        d.fill_object(&dot, orbit_color)?;
                    }
                }

                // Планета.
                let planet_obj = Object::new(
                    planet_pos,
                    Vec2::new(planet.size, planet.size),
                );

                d.fill_object(&planet_obj, planet.color)?;

                // Спутники.
                for moon in &planet.moons {
                    let moon_angle =
                        moon.start_angle + elapsed * moon.speed;

                    let moon_transform =
                        planet_transform
                        * Mat3::rotation(moon_angle)
                        * Mat3::translation(Vec2::new(
                            moon.orbit_radius,
                            0.0,
                        ));

                    let moon_pos =
                        moon_transform.transform_point(Vec2::ZERO);

                    let moon_obj = Object::new(
                        moon_pos,
                        Vec2::new(moon.size, moon.size),
                    );

                    d.fill_object(&moon_obj, moon.color)?;
                }
            }

            Ok(())
        })?;

        clock.tick();
    }
}