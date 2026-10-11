//! Рисует квадраты поверх фона окна.
//!
//! Запуск: `cargo run --example draw`

use std::time::Duration;

use frglib::{Color, Event, Rect, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib draw")
        .size(800, 600)
        .background(Color::rgb(20, 25, 30))
        .build()?;

    let mut angle = 0.0f32;

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => println!("key {key:?}"),
                _ => {}
            }
        }

        angle += 1.5;
        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);

        window.frame_with(|d| {
            // Непрозрачный квадрат в левом верхнем углу.
            d.fill_square(40.0, 40.0, 120.0, Color::rgb(220, 60, 60))?;

            // Полупрозрачный: должен смешаться с фоном, а не закрыть его.
            d.fill_rect(
                Rect::new(40.0, 200.0, 220.0, 120.0),
                Color::rgba(60, 160, 220, 128),
            )?;

            // Два непрозрачных, чтобы проверить переключение режима
            // проходимости обратно в None.
            d.fill_rect(
                Rect::new(40.0, 360.0, 220.0, 60.0),
                Color::rgb(90, 200, 120),
            )?;
            d.fill_rect(
                Rect::new(40.0, 430.0, 220.0, 60.0),
                Color::rgba(220, 180, 60, 90),
            )?;

            // Вращающийся по кругу квадрат в центре окна.
            let r = w.min(h) * 0.25;
            let cx = w / 2.0 + angle.to_radians().cos() * r;
            let cy = h / 2.0 + angle.to_radians().sin() * r;
            d.fill_square_centered(cx, cy, 60.0, Color::WHITE)?;

            Ok(())
        })?;

        std::thread::sleep(Duration::from_millis(16));
    }
}
