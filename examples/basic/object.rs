//! Три объекта: заливка, текстура и многоугольник, все с поворотом.
//!
//! Запуск: `cargo run --example basic-object`

use std::path::PathBuf;
use std::time::Instant;

use frglib::{Color, Event, FrameClock, Image, Object, Window};

/// Картинка лежит рядом с примерами; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/basic/assets")
        .join(name)
}

fn main() -> Result<(), frglib::Error> {
    let logo = Image::from_file(asset("fgoty.png"))?;

    let mut window = Window::builder("FRGLib object")
        .size(800, 600)
        .background(Color::rgb(20, 25, 30))
        .build()?;

    let side = 160.0;

    // Объект задаёт центр, размер и угол. Текстуру он не хранит, поэтому один
    // и тот же объект можно рисовать любой картинкой.
    let solid = Object::from_xywh(160.0, 300.0, side, side);
    let textured = Object::from_xywh(400.0, 300.0, side, side);
    let shaped = Object::from_xywh(640.0, 300.0, side, side);

    // Треугольник задаётся в экранных координатах: объект его только поворачивает
    // вокруг своего центра, на `size` многоугольник не смотрит.
    let triangle = [(640.0, 220.0), (720.0, 380.0), (560.0, 380.0)];

    let start = Instant::now();
    let mut clock = FrameClock::new(60.0);

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                _ => {}
            }
        }

        // Угол растёт от времени, а не от числа кадров: иначе скорость вращения
        // зависела бы от того, как быстро машина тянет кадры.
        // Угол растёт от времени, а не от числа кадров: иначе скорость вращения
        // зависела бы от того, как быстро машина тянет кадры. Это абсолютный
        // отсчёт от старта, а не шаг кадра, поэтому FrameClock тут не помогает.
        let elapsed = start.elapsed().as_secs_f32();
        let solid = solid.with_rotation(elapsed * 40.0);
        let textured = textured.with_rotation(-elapsed * 55.0);
        let shaped = shaped.with_rotation(elapsed * 70.0);

        window.frame_with(|d| {
            // 1. Повёрнутая заливка цветом: flip здесь не имеет смысла.
            d.fill_object(&solid, Color::rgba(220, 60, 60, 200))?;

            // 2. Повёрнутая текстура: поворот считает сам SDL.
            d.fill_textured_object(&textured, &logo)?;

            // 3. Тот же многоугольник, залитый и обведённый, уже повёрнутый.
            d.fill_object_polygon(&shaped, &triangle, &[0, 1, 2], Color::rgb(90, 150, 220))?;

            // 4. Углы объекта можно и обвести -- повёрнутый контур ложится на саму фигуру.
            for obj in [&solid, &textured, &shaped] {
                d.draw_object(obj, Color::rgba(255, 255, 255, 90))?;
            }

            Ok(())
        })?;

        clock.tick();
    }
}
