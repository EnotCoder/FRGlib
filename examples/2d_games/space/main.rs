use frglib::{
    Color, Event, FrameClock, Image, Keycode, Object, Vec2, Window,
};
use std::path::PathBuf;

mod bodies;
mod draw;

use bodies::*;
use draw::*;

/// Путь к картинке из общей папки ассетов.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets/space")
        .join(name)
}

fn main() -> Result<(), frglib::Error> {
    // textures
    let sun_image = Image::from_file(asset("sun.png"))?;

    let mercury_image = Image::from_file(asset("planets/mercury.png"))?;
    let vinera_image = Image::from_file(asset("planets/vinera.png"))?;
    let earth_image = Image::from_file(asset("planets/earth.png"))?;
    let mars_image = Image::from_file(asset("planets/mars.png"))?;

    let jupiter_image = Image::from_file(asset("planets/jupiter.png"))?;
    let saturn_image = Image::from_file(asset("planets/saturn.png"))?;
    let uran_image = Image::from_file(asset("planets/uran.png"))?;
    let neptune_image = Image::from_file(asset("planets/neptune.png"))?;

    // create window
    let mut window = Window::builder("FRGlib: planets and stars")
        .size(1200, 900)
        .pixel_art(true)
        .background(Color::rgb(20, 20, 25))
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut elapsed = 0.0f32;

    let orbit_thickness = 2.0;
    let orbit_color = Color::rgb(80, 80, 85);

    let planets = get_planets(mercury_image, vinera_image, 
        earth_image, mars_image, jupiter_image, 
        saturn_image, uran_image, neptune_image);

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

            d.fill_textured_object(&sun, &sun_image)?;

            // Планеты

            let _ = draw_bodies(d, &planets, elapsed, 
                center, orbit_thickness, orbit_color);

            Ok(())
        })?;

        clock.tick();
    }
}