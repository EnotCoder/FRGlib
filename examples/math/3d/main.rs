use frglib::{
    Color, Event, FrameClock, Keycode, Object, Window, Vec2,
};

mod mat;
mod transform;
mod points;

use mat::*;
use points::*;
use transform::*;

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGlib: rotating 3D polygon")
        .size(900, 700)
        .background(Color::rgb(18, 20, 30))
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut elapsed = 0.0f32;
        
    // Переменные для отрисовки
    
    let vertices = get_rect_vert(130.0);

    let camera_distance = 600.0;
    let focal_length = 600.0;
    let indices = [0, 1, 2, 0, 2, 3];

    loop {
        elapsed += clock.dt();
        
        // Событие для выхода
        
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => {
                    return Ok(());
                }

                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => {
                    return Ok(());
                }

                _ => {}
            }
        }
        
        // Переменные канваса
        let (width, height) = window.size();

        let center = Vec2::new(
            width as f32 / 2.0,
            height as f32 / 2.0,
        );

        // Переменные transform у объектов
        let angle = elapsed;
        let offset = elapsed.sin() * 100.0;
            
        let model = translation(0.0, offset, -offset) * rotation(0.0, 0.0, angle);
            
        // Создание объекта полигона
        let mut screen_points = Vec::with_capacity(vertices.len());

        for &vertex in &vertices {
            let transformed = transform(model, vertex);

            let p = project(
                transformed,
                center,
                camera_distance,
                focal_length,
            );

            screen_points.push((p.x, p.y));
        }

        let dummy_obj = Object::new(Vec2::ZERO, Vec2::ZERO);

        // Отрисовка
    
        window.frame_with(|d| {
            d.fill_object_polygon(
                &dummy_obj,
                &screen_points,
                &indices,
                Color::rgb(100, 170, 255),
            )?;

            Ok(())
        })?;

        clock.tick();
    }
}
