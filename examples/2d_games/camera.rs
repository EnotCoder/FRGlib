//! Камера: ездит по сетке, зумит, и мышь знает, где она в мире.
//!
//! WASD или стрелки — движение камеры, колесо мыши — зум.
//!
//! Запуск: `cargo run --example 2d_games-camera`

use std::time::{Duration, Instant};

use frglib::{Camera, Color, Event, Keycode, Object, Vec2, Window};

/// Шаг сетки в единицах мира.
const CELL: f32 = 100.0;

/// Сколько клеток в сторону от центра мира.
const RADIUS: i32 = 12;

/// Скорость камеры в единицах мира в секунду.
const SPEED: f32 = 600.0;

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib camera")
        .size(800, 600)
        .background(Color::rgb(18, 20, 26))
        .build()?;

    let mut camera = Camera::new(Vec2::ZERO);

    // Сетка -- это просто объекты в координатах мира. Она нужна лишь чтобы было
    // видно, куда едет камера; никакого тайлового мира в библиотеке нет.
    let cell = CELL - 12.0;
    let grid: Vec<Object> = (-RADIUS..=RADIUS)
        .flat_map(|gy| {
            (-RADIUS..=RADIUS)
                .map(move |gx| Object::from_xywh(gx as f32 * CELL, gy as f32 * CELL, cell, cell))
        })
        .collect();

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut cursor: Option<Vec2> = None;
    let mut last = Instant::now();

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),

                Event::KeyDown { key, .. } => {
                    if !pressed.contains(&key) {
                        pressed.push(key);
                    }
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != key),

                Event::MouseMotion { x, y } => cursor = Some(Vec2::new(x, y)),

                // Зум прикручиваем к позиции курсора, иначе мир уезжает из-под
                // мыши -- наводишь на край и зум, а точка под мышью другая.
                Event::MouseWheel { y, .. } => {
                    let size = window.size();
                    let before = camera.to_world(cursor.unwrap_or(Vec2::ZERO), size);
                    camera.zoom = (camera.zoom * (1.0 + y * 0.1)).clamp(0.2, 4.0);
                    let after = camera.to_world(cursor.unwrap_or(Vec2::ZERO), size);
                    camera.position += before - after;
                }

                _ => {}
            }
        }

        let now = Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;

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

        // Границ нет: камера свободно уезжает за сетку, и это видно -- пустое
        // поле вокруг. Ограничение добавить несложно, но в этой версии его нет.
        camera.position += direction.normalized() * SPEED * dt;

        let size = window.size();
        let cursor_world = cursor.map(|c| camera.to_world(c, size));

        let marker = cursor_world.map(|p| Object::from_xywh(p.x, p.y, 24.0, 24.0));

        window.frame_with(|d| {
            // Камера -- первой строкой: всё, что нарисовано дальше, попадает на
            // экран через неё.
            d.set_camera(camera);

            for object in &grid {
                // Раскраска по клеткам, чтобы движение было видно глазом.
                let odd = ((object.position.x + object.position.y) / CELL) as i32 % 2 == 0;
                let colour = if odd {
                    Color::rgb(58, 72, 92)
                } else {
                    Color::rgb(44, 54, 70)
                };
                d.fill_object(object, colour)?;
            }

            // Метка под курсором: та же точка мира, но нарисованная объектом.
            if let Some(marker) = &marker {
                d.draw_object(marker, Color::rgb(240, 120, 60))?;
            }

            Ok(())
        })?;

        std::thread::sleep(Duration::from_millis(16));
    }
}
