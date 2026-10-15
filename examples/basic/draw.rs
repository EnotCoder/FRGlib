//! Рисует объекты поверх фона окна.
//!
//! Запуск: `cargo run --example basic-draw`

use frglib::{Color, Event, FrameClock, Object, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib draw")
        .size(800, 600)
        .background(Color::rgb(20, 25, 30))
        .build()?;

    let margin = 60.0;

    let mut clock = FrameClock::new(60.0);

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

        let avail_w = (w - margin * 2.0).max(1.0);
        let avail_h = (h - margin * 2.0).max(1.0);
        let (mut side, mut other) = (avail_w, avail_w);
        if other > avail_h {
            other = avail_h;
            side = avail_h;
        }

        // Объект задаёт центр, поэтому вписываем квадрат по центру окна, а не по
        // левому верхнему углу -- иначе при ресайзе он уезжал бы за край.
        let square = Object::from_xywh(w / 2.0, h / 2.0, side, other);

        window.frame_with(|d| {
            // Непрозрачный квадрат по центру окна.
            d.fill_object(&square, Color::rgb(220, 60, 60))?;
            Ok(())
        })?;

        clock.tick();
    }
}
