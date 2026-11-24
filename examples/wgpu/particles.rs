//! Частицы на `wgpu`: тот же симулятор, другой способ рисования.
//!
//! Запуск: `cargo run --example wgpu-particles --features wgpu`
//!
//! Система частиц не знает ни о SDL, ни о `wgpu`: здесь она рисуется через
//! [`ParticleSystem::draw_gpu`] в кадре `Renderer2D`. Фонтан следует за мышью,
//! клик и Space дают вспышку, R очищает обе системы, Escape закрывает окно.

use std::error::Error;
use std::f32::consts::FRAC_PI_2;

use frglib::gpu::{Renderer2D, WgpuWindow};
use frglib::{
    Color, Emitter, Event, FrameClock, Keycode, MouseButton, ParticleShape, ParticleSystem, Vec2,
};

const FONT: &[u8] = include_bytes!("../ui/assets/font.otf");

fn main() -> Result<(), Box<dyn Error>> {
    let mut window = WgpuWindow::builder("FRGLib particles — wgpu")
        .size(900, 600)
        .background(Color::FON)
        .build()?;

    let mut ui = Renderer2D::new(window.gpu());
    let font = ui.load_font(FONT.to_vec())?;

    let (width, height) = window.size();

    let mut fountain = ParticleSystem::with_emitter(
        1200,
        Emitter::new()
            .rate(100.0)
            .at(Vec2::new(width as f32 * 0.5, height as f32 - 70.0))
            .direction(-FRAC_PI_2)
            .spread(1.0)
            .speed(160.0, 320.0)
            .lifetime(0.9, 1.8)
            .gravity(Vec2::new(0.0, 300.0))
            .drag(0.3)
            .size(5.0, 11.0)
            .end_scale(0.1)
            .colors(Color::rgba(255, 206, 120, 255), Color::rgba(246, 86, 64, 0))
            .shape(ParticleShape::Circle),
    );

    let mut burst = ParticleSystem::with_emitter(
        1500,
        Emitter::new()
            .speed(70.0, 300.0)
            .lifetime(0.5, 1.2)
            .gravity(Vec2::new(0.0, 150.0))
            .drag(0.5)
            .size(4.0, 9.0)
            .end_scale(0.0)
            .colors(
                Color::rgba(146, 228, 255, 255),
                Color::rgba(86, 106, 244, 0),
            )
            .shape(ParticleShape::Square),
    );

    let mut clock = FrameClock::new(60.0);

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => return Ok(()),
                Event::MouseMotion { x, y } => {
                    fountain.emitter_mut().origin = Vec2::new(x, y);
                }
                Event::MouseButtonDown {
                    button: MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    burst.emit_at(120, Vec2::new(x, y));
                }
                Event::KeyDown {
                    key: Keycode::Space,
                    ..
                } => {
                    burst.emit_at(200, Vec2::new(width as f32 * 0.5, height as f32 * 0.5));
                }
                Event::KeyDown {
                    key: Keycode::R, ..
                } => {
                    fountain.clear();
                    burst.clear();
                }
                _ => {}
            }
        }

        let dt = clock.dt();
        fountain.update(dt);
        burst.update(dt);

        let Some(mut frame) = window.begin_frame()? else {
            clock.tick();
            continue;
        };

        ui.begin(&frame);

        fountain.draw_gpu(&mut ui);
        burst.draw_gpu(&mut ui);

        ui.draw_text(
            font,
            "FRGLib particles / wgpu",
            24.0,
            24.0,
            22.0,
            Color::WHITE,
        )?;
        ui.draw_text(
            font,
            "мышь — фонтан, клик — вспышка, Space — в центре, R — очистить",
            24.0,
            54.0,
            15.0,
            Color::rgb(170, 186, 210),
        )?;

        ui.finish(&mut frame);
        window.present(frame)?;
        clock.tick();
    }
}
