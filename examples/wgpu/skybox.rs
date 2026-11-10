//! Skybox: кубическое небо и простая сцена на `wgpu`.
//!
//! Запуск: `cargo run --example wgpu-skybox --features wgpu`
//!
//! Панорама 2:1 генерируется процедурно (градиент неба, земля и солнце), затем
//! `Skybox` преобразует её в кубическую текстуру. Камера медленно облетает
//! сцену, поэтому видно, что небо поворачивается вместе с ней.

use std::f32::consts::{PI, TAU};

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, Renderer3D, Skybox, WgpuWindow};
use frglib::{Color, Event, FrameClock};

fn main() -> Result<(), frglib::gpu::Error> {
    let mut window = WgpuWindow::builder("FRGLib skybox")
        .size(900, 600)
        .background(Color::rgb(10, 12, 20))
        .build()?;

    let mut renderer = Renderer3D::new(window.gpu());

    // Панорама снизу вверх: земля, горизонт, зенит и солнечное пятно.
    let (width, height, pixels) = panorama(1024, 512);
    let skybox = Skybox::from_equirectangular_rgba8(window.gpu(), width, height, &pixels)?;
    renderer.set_skybox(skybox);

    let mut clock = FrameClock::new(60.0);
    let mut time = 0.0f32;

    loop {
        for event in window.poll()? {
            if matches!(event, Event::Quit | Event::CloseRequested) {
                return Ok(());
            }
        }

        time += clock.dt();

        let Some(mut frame) = window.begin_frame()? else {
            clock.tick();
            continue;
        };

        let camera = Camera3D::orbit(Vec3::new(0.0, 1.0, 0.0), 7.5, time * 0.35, 0.28);

        renderer.begin(&camera);
        renderer.draw_grid(24, 1.0, Color::rgb(70, 92, 110));

        // Несколько колонн по кругу; их высота и цвет меняются, чтобы сцена
        // не выглядела плоской.
        for i in 0..6 {
            let angle = i as f32 / 6.0 * TAU;
            let height = 0.5 + (i % 3) as f32 * 0.6;
            let position = Vec3::new(angle.cos() * 2.4, height * 0.5, angle.sin() * 2.4);
            let model = Mat4::from_translation(position)
                * Mat4::from_rotation_y(angle + time)
                * Mat4::from_scale(Vec3::splat(0.8));
            let color = Color::rgb(160 + i as u8 * 12, 110, 90 + i as u8 * 18);
            renderer.draw_cube_model(model, color);
        }

        renderer.draw_cube(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.6, 2.0, 1.6),
            Color::rgb(190, 200, 210),
        );

        renderer.finish(&mut frame);
        window.present(frame)?;

        clock.tick();
    }
}

/// Renders a 2:1 equirectangular sky: ground below the horizon, a blue gradient
/// above it, and a warm sun.
fn panorama(width: u32, height: u32) -> (u32, u32, Vec<u8>) {
    let sun = Vec3::new(0.35, 0.22, -0.91).normalize();
    let horizon = Vec3::new(0.78, 0.84, 0.94);
    let zenith = Vec3::new(0.08, 0.20, 0.52);
    let ground = Vec3::new(0.06, 0.08, 0.11);
    let sun_color = Vec3::new(1.0, 0.88, 0.66);

    let mut pixels = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        let v = (y as f32 + 0.5) / height as f32;
        let elevation = (0.5 - v) * PI;

        for x in 0..width {
            let u = (x as f32 + 0.5) / width as f32;
            let azimuth = (u - 0.5) * TAU;
            let direction = Vec3::new(
                elevation.cos() * azimuth.sin(),
                elevation.sin(),
                elevation.cos() * azimuth.cos(),
            );

            let mut color = if elevation >= 0.0 {
                let t = (elevation / (PI * 0.5)).clamp(0.0, 1.0);
                horizon.lerp(zenith, t.powf(0.7))
            } else {
                let t = (-elevation / (PI * 0.5)).clamp(0.0, 1.0);
                horizon.lerp(ground, t.powf(0.45))
            };

            let sun_dot = direction.dot(sun).max(0.0);
            color += sun_color * (sun_dot.powf(256.0) * 1.4 + sun_dot.powf(12.0) * 0.12);

            pixels.extend_from_slice(&[
                (color.x.clamp(0.0, 1.0) * 255.0).round() as u8,
                (color.y.clamp(0.0, 1.0) * 255.0).round() as u8,
                (color.z.clamp(0.0, 1.0) * 255.0).round() as u8,
                255,
            ]);
        }
    }

    (width, height, pixels)
}
