//! Куб, сетка и оси под вращающейся камерой на `wgpu`.
//!
//! Запуск: `cargo run --example wgpu-cube --features wgpu`
//!
//! Стрелки крутят камеру вокруг сцены, Escape закрывает окно. Помимо
//! готового [`Renderer3D`] пример ничего не использует: тот же кадр можно
//! дополнить своими проходами через [`frglib::gpu::Frame`].

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, Renderer3D, WgpuWindow};
use frglib::{Color, Event, FrameClock, Keycode};

fn main() -> Result<(), frglib::gpu::Error> {
    let mut window = WgpuWindow::builder("FRGLib wgpu")
        .size(900, 600)
        .background(Color::FON)
        .build()?;

    let info = window.gpu().adapter_info();
    println!("{} ({:?} {})", info.name, info.backend, info.driver);

    let mut renderer = Renderer3D::new(window.gpu());
    let mut clock = FrameClock::new(60.0);
    let mut time = 0.0f32;
    let yaw = 0.7f32;
    let pitch = 0.35f32;

    loop {
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

        time += clock.dt();

        let Some(mut frame) = window.begin_frame()? else {
            // Свёрнутое окно или поверхность не готова: кадр пропускается.
            clock.tick();
            continue;
        };

        let camera = Camera3D::orbit(Vec3::new(0.0, 0.7, 0.0), 6.0, yaw + time * 0.5, pitch);

        renderer.begin(&camera);
        renderer.draw_grid(20, 1.0, Color::rgb(70, 82, 94));

        let cube = Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0))
            * Mat4::from_rotation_y(15.0)
            * Mat4::from_scale(Vec3::splat(1.5));
        renderer.draw_cube_model(cube, Color::rgb(146, 197, 141));

        renderer.finish(&mut frame);
        window.present(frame)?;

        clock.tick();
    }
}
