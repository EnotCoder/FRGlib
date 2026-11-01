use frglib::{
    Color, Event, FrameClock, Keycode, Object, Vec2, Window,
};

mod bodies;
mod draw;

use bodies::*;
use draw::*;

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGlib: planets and stars")
        .size(1200, 900)
        .background(Color::rgb(20, 20, 25))
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut elapsed = 0.0f32;

    let orbit_thickness = 2.0;
    let orbit_color = Color::rgb(80, 80, 85);

    let planets = get_planets();

    // Создаём звёзды один раз.
    let stars = get_stars();

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
            let _ = draw_stars(d, &stars, &window_size);

            // Орбиты планет.
            let _ = draw_orbit(d, &planets, 
                center, orbit_thickness, orbit_color);

            // Солнце.
            let sun = Object::new(
                center,
                Vec2::new(35.0, 35.0),
            );

            d.fill_object(&sun, Color::rgb(255, 200, 50))?;

            // Планеты

            let _ = draw_bodies(d, &planets, elapsed, 
                center, orbit_thickness, orbit_color);

            Ok(())
        })?;

        clock.tick();
    }
}