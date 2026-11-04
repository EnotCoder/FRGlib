//! Пример использования AABB коллизий.
//!
//! Два прямоугольника: один управляется мышью, другой стоит на месте.
//! Когда они пересекаются, их цвет меняется.

use frglib::{Color, Event, Rect, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib AABB Example")
        .size(800, 600)
        .background(Color::rgb(30, 30, 35))
        .build()?;

    let mut mouse_pos = (0.0, 0.0);
    let static_rect = Rect::new(350.0, 250.0, 100.0, 100.0);

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::MouseMotion { x, y, .. } => {
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

        window.frame_with(|d| {
            d.fill_rect(static_rect, Color::rgb(150, 150, 150))?;
            d.fill_rect(player_rect, player_color)?;
            Ok(())
        })?;
    }
}
