//! Частицы: непрерывный фонтан и вспышки по клику.
//!
//! Запуск: `cargo run --example basic-particles`
//!
//! Две системы живут в одном кадре. Первая всё время сыплет частицы из точки,
//! за которой следует курсор; вторая молчит до нажатия и выдаёт вспышку сразу
//! большой пачкой. Обе рисуются через `ParticleSystem::draw` в текущей
//! системе координат окна.
//!
//! Управление: двигать мышь — перенести фонтан, левый клик — вспышка,
//! Space — вспышка в центре, R — очистить, Escape — выйти.

use std::f32::consts::FRAC_PI_2;

use frglib::{
    Color, Emitter, Event, FrameClock, Keycode, MouseButton, ParticleShape, ParticleSystem, Vec2,
    Window,
};

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGLib particles")
        .size(900, 600)
        .background(Color::FON)
        .build()?;

    let (width, height) = window.size();

    let mut fountain = ParticleSystem::with_emitter(
        1200,
        Emitter::new()
            .rate(90.0)
            .at(Vec2::new(width as f32 * 0.5, height as f32 - 70.0))
            // -PI/2 смотрит вверх по экрану, разброс в 0.7 радиан даёт конус.
            .direction(-FRAC_PI_2)
            .spread(0.7)
            .speed(180.0, 340.0)
            .lifetime(0.8, 1.7)
            .gravity(Vec2::new(0.0, 380.0))
            .drag(0.25)
            .size(4.0, 9.0)
            .end_scale(0.15)
            .colors(Color::rgba(255, 214, 130, 255), Color::rgba(232, 78, 60, 0))
            .shape(ParticleShape::Circle),
    );

    let mut burst = ParticleSystem::with_emitter(
        1500,
        Emitter::new()
            // rate = 0: частицы появляются только вызовами emit_at.
            .speed(60.0, 280.0)
            .lifetime(0.5, 1.1)
            .gravity(Vec2::new(0.0, 160.0))
            .drag(0.5)
            .size(3.0, 7.0)
            .end_scale(0.0)
            .colors(
                Color::rgba(150, 226, 255, 255),
                Color::rgba(80, 100, 240, 0),
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
                    let (w, h) = window.size();
                    burst.emit_at(200, Vec2::new(w as f32 * 0.5, h as f32 * 0.5));
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

        window.frame_with(|d| {
            fountain.draw(d)?;
            burst.draw(d)?;
            Ok(())
        })?;

        clock.tick();
    }
}
