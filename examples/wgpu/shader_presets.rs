//! Все готовые шейдеры в одном кадре: мультяшный, комикс, ретро и другие.
//!
//! Запуск: `cargo run --example wgpu-shader-presets --features wgpu`
//!
//! Клавиши 1–8 оставляют только выбранный стиль, Space показывает все сразу,
//! Escape закрывает окно. Материалы компилируются один раз до цикла кадров.

use frglib::gpu::glam::{Mat4, Vec3};
use frglib::gpu::{Camera3D, Light, Renderer3D, ShaderPreset, WgpuWindow};
use frglib::{Color, Event, FrameClock, Keycode};

fn main() -> Result<(), frglib::gpu::Error> {
    let mut window = WgpuWindow::builder("FRGLib shader presets")
        .size(1100, 600)
        .background(Color::rgb(22, 24, 32))
        .build()?;

    let mut renderer = Renderer3D::new(window.gpu());
    renderer.set_lights([
        Light::directional(Vec3::new(-0.5, -1.0, -0.6), Color::WHITE).with_intensity(1.2),
        Light::point(Vec3::new(0.0, 1.5, 3.0), Color::rgb(120, 180, 255))
            .with_range(9.0)
            .with_intensity(0.6),
    ]);
    renderer.set_ambient(Color::rgb(48, 48, 55));

    // Компиляция WGSL — не самая дешёвая операция, поэтому делаем её заранее.
    let materials: Vec<_> = ShaderPreset::ALL
        .iter()
        .map(|preset| renderer.create_preset_material(*preset))
        .collect();

    println!("1..8 — выбрать стиль, Space — показать все:");
    for (index, preset) in ShaderPreset::ALL.iter().enumerate() {
        println!("  {} — {}", index + 1, preset.name());
    }

    let mut clock = FrameClock::new(60.0);
    let mut time = 0.0f32;
    let mut focus: Option<usize> = None;

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Space,
                    ..
                } => focus = None,
                Event::KeyDown { key, .. } => {
                    let digit = match key {
                        Keycode::_1 => Some(1),
                        Keycode::_2 => Some(2),
                        Keycode::_3 => Some(3),
                        Keycode::_4 => Some(4),
                        Keycode::_5 => Some(5),
                        Keycode::_6 => Some(6),
                        Keycode::_7 => Some(7),
                        Keycode::_8 => Some(8),
                        _ => None,
                    };
                    if let Some(digit) = digit {
                        focus = Some(digit - 1);
                        println!("Стиль: {}", ShaderPreset::ALL[digit - 1].name());
                    }
                }
                _ => {}
            }
        }

        time += clock.dt();
        let Some(mut frame) = window.begin_frame()? else {
            clock.tick();
            continue;
        };

        let camera = Camera3D::orbit(Vec3::new(0.0, 0.8, 0.0), 10.0, 0.15, 0.30);
        renderer.begin(&camera);
        renderer.draw_grid(20, 1.0, Color::rgb(55, 62, 74));

        let count = materials.len() as f32;
        for (index, material) in materials.iter().enumerate() {
            if focus.is_some_and(|chosen| chosen != index) {
                continue;
            }
            let x = (index as f32 - (count - 1.0) * 0.5) * 1.8;
            let model = Mat4::from_translation(Vec3::new(x, 0.9, 0.0))
                * Mat4::from_rotation_y(time * 0.6 + index as f32 * 0.3)
                * Mat4::from_scale(Vec3::splat(0.75));
            renderer.draw_cube_model_with(model, *material, Color::rgb(150, 200, 170));
        }

        renderer.finish(&mut frame);
        window.present(frame)?;
        clock.tick();
    }
}
