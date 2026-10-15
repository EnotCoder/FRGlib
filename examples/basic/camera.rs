//! Камера: ездит по сцене и зумит.
//!
//! Стрелки или WASD — движение камеры, колесо мыши — зум к курсору.
//!
//! Запуск: `cargo run --example basic-camera`

use frglib::{Camera, Color, Event, FrameClock, Image, Keycode, Object, Vec2, Window};
use std::path::PathBuf;

/// Картинка лежит рядом с примерами; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/basic/assets")
        .join(name)
}

/// Скорость камеры в единицах мира в секунду.
const SPEED: f32 = 600.0;

fn main() -> Result<(), frglib::Error> {
    let frog = Image::from_file(asset("fgoty_with_body.png"))?;

    let mut window = Window::builder("FRGLib camera")
        .size(800, 600)
        .background(Color::FON)
        .build()?;

    let mut camera = Camera::new(Vec2::ZERO);

    // Картинка 800x600, а на экране ей полагается быть вдвое меньше: в мире
    // размер считается в единицах мира, и при зуме 1 это ровно пиксели. Стоит
    // она в начале координат, то есть ровно там, где стоит камера.
    let (iw, ih) = frog.size();
    let hero = Object::from_xywh(0.0, 0.0, iw as f32 / 2.0, ih as f32 / 2.0);

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut cursor: Option<Vec2> = None;
    let mut clock = FrameClock::new(60.0);

    loop {
        // Сколько секунд длился прошлый кадр -- ровно то, чем двигать скорость.
        let dt = clock.dt();

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),

                Event::KeyDown { key, .. } => {
                    if !pressed.contains(&key) {
                        pressed.push(key);
                    }
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != key),

                Event::MouseMotion { x, y } => cursor = Some(Vec2::new(x, y)),

                // Зум прикручиваем к позиции курсора, иначе мир уезжает из-под
                // мыши -- наводишь на край и зум, а точка под мышью другая.
                Event::MouseWheel { y, .. } => {
                    let size = window.size();
                    let before = camera.to_world(cursor.unwrap_or(Vec2::ZERO), size);
                    camera.zoom = (camera.zoom * (1.0 + y * 0.1)).clamp(0.2, 4.0);
                    let after = camera.to_world(cursor.unwrap_or(Vec2::ZERO), size);
                    camera.position += before - after;
                }

                _ => {}
            }
        }

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

        // Границ нет: камера свободно уезжает за сцену, и это видно -- пустое поле
        // вокруг. Ограничение добавить несложно, но в этой версии его нет.
        camera.position += direction.normalized() * SPEED * dt;

        window.frame_with(|d| {
            // Камера -- первой строкой: всё, что нарисовано дальше, попадает на
            // экран через неё.
            d.set_camera(camera);

            d.fill_textured_object(&hero, &frog)?;

            Ok(())
        })?;

        clock.tick();
    }
}
