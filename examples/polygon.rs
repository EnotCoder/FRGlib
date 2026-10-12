//! Заполняет многоугольник по индексам, обводит его контур и накладывает
//! текстуру на второй многоугольник.
//!
//! Запуск: `cargo run --example polygon`

use std::path::PathBuf;
use std::time::Duration;

use frglib::{Color, Event, Image, Window};

/// Логотип лежит в репозитории; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/assets")
        .join(name)
}

fn main() -> Result<(), frglib::Error> {
    let logo = Image::from_file(asset("fgoty.png"))?;

    let mut window = Window::builder("FRGLib draw")
        .size(800, 600)
        .background(Color::rgb(20, 25, 30))
        .build()?;

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => println!("key {key:?}"),
                _ => {}
            }
        }

        let points = [
            (100.0, 100.0), // 0 — A
            (300.0, 80.0),  // 1 — B
            (350.0, 250.0), // 2 — C
            (120.0, 300.0), // 3 — D
        ];

        let indices = [
            0, 1, 2, // Первый треугольник: A → B → C.
            0, 2, 3, // Второй треугольник: A → C → D.
        ];

        // Второй многоугольник, поменьше и правее: текстура растягивается по
        // его bounding box, поэтому пропорции картинки тут не сохраняются.
        let text_points = [
            (300.0, 150.0),
            (690.0, 300.0),
            (640.0, 520.0),
            (470.0, 470.0),
        ];

        let text_indices = [
            0, 1, 2, // Треугольник A → B → C.
            0, 2, 3, // Треугольник A → C → D.
        ];

        window.frame_with(|d| {
            // Сначала заливка.
            d.fill_polygon(
                &points,
                &indices,
                Color::rgb(220, 60, 60),
            )?;

            // А здесь — тот же многоугольник, но залитый текстурой.
            d.fill_textured_polygon(&text_points, &text_indices, &logo)?;

            Ok(())
        })?;

        std::thread::sleep(Duration::from_millis(16));
    }
}
