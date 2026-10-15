//! Текстура, залитая один раз, против залитой на каждом кадре.
//!
//! Слева жаба рисуется через `Image` -- картинка уезжает на GPU каждый кадр.
//! Справа та же жаба через `Texture`, залитая один раз при старте.
//!
//! Запуск: `cargo run --features cached-textures --example basic-cached`

use frglib::{Camera, Color, Event, FrameClock, Image, Keycode, Object, Vec2, Window};
use std::path::PathBuf;

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/basic/assets")
        .join(name)
}

/// Скорость камеры в единицах мира в секунду.
const SPEED: f32 = 600.0;

fn main() -> Result<(), frglib::Error> {
    let frog = Image::from_file(asset("fgoty_with_body.png"))?;

    let mut window = Window::builder("FRGLib cached")
        .size(800, 600)
        .background(Color::FON)
        .build()?;

    // Вот и вся разница: одна загрузка сейчас, а не шестьдесят раз в секунду.
    // Ручка берёт адресность у окна и потому не может пережить его.
    let frog_gpu = window.upload(&frog)?;
    println!("текстура: {frog_gpu:?}");

    let mut camera = Camera::new(Vec2::ZERO);

    let (iw, ih) = frog.size();
    let (w, h) = (iw as f32 / 2.0, ih as f32 / 2.0);

    let slow = Object::from_xywh(-260.0, 0.0, w, h);
    let fast = Object::from_xywh(260.0, 0.0, w, h);

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => {
                    if !pressed.contains(&key) {
                        pressed.push(key);
                    }
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != key),
                _ => {}
            }
        }

        let dt = clock.dt();

        let held = |key| pressed.contains(&key);
        let mut direction = Vec2::ZERO;
        for (keys, step) in [
            ([Keycode::Left, Keycode::A], Vec2::new(-1.0, 0.0)),
            ([Keycode::Right, Keycode::D], Vec2::new(1.0, 0.0)),
            ([Keycode::Up, Keycode::W], Vec2::new(0.0, -1.0)),
            ([Keycode::Down, Keycode::S], Vec2::new(0.0, 1.0)),
        ] {
            if keys.iter().any(|&key| held(key)) {
                direction += step;
            }
        }
        camera.position += direction.normalized() * SPEED * dt;

        window.frame_with(|d| {
            d.set_camera(camera);

            // Заливка на каждом кадре.
            d.fill_textured_object(&slow, &frog)?;

            // Одна заливка при старте.
            d.fill_object_texture(&fast, frog_gpu)?;

            Ok(())
        })?;

        clock.tick();
    }
}
