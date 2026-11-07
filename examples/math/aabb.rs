//! Пример использования AABB коллизий.
//!
//! Два прямоугольника: один управляется мышью, другой стоит на месте.
//! Когда они пересекаются, цвет игрока меняется.
//!
//! Запуск: `cargo run --example math-aabb`

use frglib::{Color, Event, FrameClock, Keycode, Object, Rect, Window};

/// `Object` рисуется от центра, а `Rect` хранит левый верхний угол.
fn object_of(rect: Rect) -> Object {
    Object::from_xywh(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0, rect.w, rect.h)
}

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib AABB Example")
        .size(800, 600)
        .background(Color::rgb(30, 30, 35))
        .build()?;

    let mut clock = FrameClock::new(60.0);

    // Стартуем в центре окна, а не в углу.
    let mut mouse_pos = (400.0, 300.0);
    let static_rect = Rect::new(350.0, 250.0, 100.0, 100.0);
    let static_obj = object_of(static_rect);

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                Event::MouseMotion { x, y } => {
                    mouse_pos = (x, y);
                }
                _ => {}
            }
        }

        let player_rect = Rect::square_centered(mouse_pos.0, mouse_pos.1, 60.0);
        let colliding = player_rect.overlaps(&static_rect);

        let player_color = if colliding {
            Color::rgb(255, 100, 100)
        } else {
            Color::rgb(100, 255, 100)
        };

        let player_obj = object_of(player_rect);

        window.frame_with(|d| {
            d.fill_object(&static_obj, Color::rgb(150, 150, 150))?;
            d.fill_object(&player_obj, player_color)?;
            Ok(())
        })?;

        clock.tick();
    }
}
