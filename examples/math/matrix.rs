//! Матрицы: иерархические трансформации.
//!
//! Пример показывает, как с помощью матриц 3x3 (`Mat3`) создавать вложенные
//! системы координат. "Планета" вращается вокруг центра экрана, а "спутник"
//! вращается вокруг планеты.
//!
//! Управление: Escape -- выход.
//!
//! Запуск: `cargo run --example math-matrix`

use frglib::{Color, Event, FrameClock, Keycode, Mat3, Object, Vec2, Window};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib Matrix Hierarchy")
        .size(800, 600)
        .background(Color::rgb(20, 20, 25))
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut elapsed = 0.0f32;

    loop {
        let dt = clock.dt();
        elapsed += dt;

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                _ => {}
            }
        }

        let center = {
            let (w, h) = window.size();
            Vec2::new(w as f32 / 2.0, h as f32 / 2.0)
        };

        // 1. Матрица планеты: вращение + перенос от центра
        let planet_orbit_radius = 150.0;
        let planet_angle = elapsed * 1.0; // 1 рад/сек
        
        let planet_transform = Mat3::translation(center) 
            * Mat3::rotation(planet_angle)
            * Mat3::translation(Vec2::new(planet_orbit_radius, 0.0));

        let planet_pos = planet_transform.transform_point(Vec2::ZERO);
        let planet_obj = Object::new(planet_pos, Vec2::new(40.0, 40.0));

        // 2. Матрица спутника: относительна планеты
        let moon_orbit_radius = 60.0;
        let moon_angle = elapsed * 3.0; // вращается быстрее

        // Матрица спутника строится поверх матрицы планеты
        let moon_transform = planet_transform 
            * Mat3::rotation(moon_angle) 
            * Mat3::translation(Vec2::new(moon_orbit_radius, 0.0));

        let moon_pos = moon_transform.transform_point(Vec2::ZERO);
        let moon_obj = Object::new(moon_pos, Vec2::new(15.0, 15.0));

        window.frame_with(|d| {
            // Центр системы (Солнце)
            d.fill_object(
                &Object::new(center, Vec2::new(60.0, 60.0)),
                Color::rgb(255, 200, 50),
            )?;

            // Планета
            d.fill_object(&planet_obj, Color::rgb(100, 150, 255))?;

            // Спутник
            d.fill_object(&moon_obj, Color::rgb(200, 200, 200))?;

            // Вывод определителя в консоль раз в секунду
            if elapsed as i32 % 5 == 0 && (elapsed - elapsed.floor()) < 0.016 {
                println!("Planet matrix determinant: {}", planet_transform.determinant());
            }

            Ok(())
        })?;

        clock.tick();
    }
}