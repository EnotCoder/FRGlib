//! Пример использования функций sin и cos для кругового движения.
//!
//! Запуск: `cargo run --example basic-sin-cos`

use frglib::{cos, sin, Color, Event, FrameClock, Object, Vec2, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib sin/cos")
        .size(800, 600)
        .background(Color::FON)
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut time = 0.0f32;

    loop {
        let dt = clock.dt();
        time += dt;

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                _ => {}
            }
        }

        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);
        
        let center = Vec2::new(w / 2.0, h / 2.0);
        let radius = 150.0;
        
        // Используем sin и cos для вычисления позиции на круге
        let x = center.x + cos(time * 2.0) * radius;
        let y = center.y + sin(time * 2.0) * radius;
        
        let square = Object::from_xywh(x, y, 40.0, 40.0);

        window.frame_with(|d| {
            d.fill_object(&square, Color::RED)?;
            Ok(())
        })?;

        clock.tick();
    }
}
