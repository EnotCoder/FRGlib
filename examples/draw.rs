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


    let margin = 60.0;

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => println!("key {key:?}"),
                _ => {}
            }
        }

        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);

        // Вписываем логотип в окно целиком, с отступами.
        let avail_w = (w - margin * 2.0).max(1.0);
        let avail_h = (h - margin * 2.0).max(1.0);
        let (mut lw, mut lh) = (avail_w, avail_w);
        if lh > avail_h {
            lh = avail_h;
            lw = avail_h;
        }

        let x = (w - lw) / 2.0;
        let y = (h - lh) / 2.0;
        

        window.frame_with(|d| {
            // Непрозрачный квадрат в левом верхнем углу.
            d.fill_rect(Rect::new(x, y, lw, lh), Color::rgb(220, 60, 60))?;
            Ok(())
        })?;

        std::thread::sleep(Duration::from_millis(16));
    }
}
