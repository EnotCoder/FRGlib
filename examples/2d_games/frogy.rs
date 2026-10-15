//! Объект, который едет по окну, крутится и отражается.
//!
//! Стрелки или WASD — движение, пробел — поворот, F — отражение по горизонтали.
//!
//! Запуск: `cargo run --example 2d_games-frogy`

use frglib::{Color, Event, FrameClock, Image, Keycode, Object, Vec2, Window};
use std::path::PathBuf;

/// Спрайты лежат рядом с примером; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/2d_games/assets/rpg_frogy")
        .join(name)
}

fn main() -> Result<(), frglib::Error> {
    let up = Image::from_file(asset("frogy_up.png"))?;
    let down = Image::from_file(asset("frogy_down.png"))?;
    let left = Image::from_file(asset("frogy_left.png"))?;
    let right = Image::from_file(asset("frogy_right.png"))?;

    let mut window = Window::builder("FRGLib frogy")
        .size(800, 600)
        .background(Color::rgb(20, 25, 30))
        .build()?;

    // Спрайт 16x16 -- растягиваем вчетверо, иначе его не разглядеть.
    let side = 64.0;
    let speed = 180.0;

    let mut frogy = Object::from_xywh(400.0, 300.0, side, side);

    // Какие клавиши сейчас зажаты. Само событие -- это «щёлкнули», поэтому
    // состояние приходится копить и отдельно снимать по отпусканию.
    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);

    loop {
        // Сколько секунд длился прошлый кадр -- ровно то, чем
        // двигать скорость.
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

        frogy.velocity = direction.normalized() * speed;
        frogy.update(dt);

        if held(Keycode::Space) {
            // Держим угол в 0..360, иначе он растёт бесконечно и теряет точность.
            frogy.rotation = (frogy.rotation + 90.0 * dt).rem_euclid(360.0);
        }
        frogy.flip_horizontal = held(Keycode::F);

        // Не выпускаем за края окна: при ресайзе окна границы тоже поедут.
        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);
        let half = frogy.size * 0.5;
        frogy.position.x = frogy.position.x.clamp(half.x, w - half.x);
        frogy.position.y = frogy.position.y.clamp(half.y, h - half.y);

        // Спрайт выбираем по направлению, но разворот по горизонтали задаётся
        // отдельно -- так виден и сам flip, а не только то, что мы едем влево.
        let sprite = if direction.length_squared() < f32::EPSILON {
            &down
        } else if direction.y.abs() > direction.x.abs() {
            if direction.y < 0.0 {
                &up
            } else {
                &down
            }
        } else if direction.x < 0.0 {
            &left
        } else {
            &right
        };

        window.frame_with(|d| {
            d.fill_textured_object(&frogy, sprite)?;
            Ok(())
        })?;

        clock.tick();
    }
}
