//! Заполняет многоугольник по индексам и обводит его контур.
//!
//! Запуск: `cargo run --example polygon`

use std::time::Duration;

use frglib::{Color, Event, Window};

fn main() -> Result<(), frglib::Error> {
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

        window.frame_with(|d| {
            // Сначала заливка.
            d.fill_polygon(
                &points,
                &indices,
                Color::rgb(220, 60, 60),
            )?;

            // Затем контур поверх неё.
            d.draw_polygon(
                &points,
                Color::WHITE,
            )?;

            Ok(())
        })?;

        std::thread::sleep(Duration::from_millis(16));
    }
}
