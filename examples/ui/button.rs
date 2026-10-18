//! Кнопки: три состояния и клик по отпусканию.
//!
//! Подписи рисует SDL3_ttf, а кнопка принимает их готовыми картинками и
//! ничего про шрифты не знает. Поэтому в библиотеке ни SDL3_ttf, ни
//! `ab_glyph`, ни `rusttype` нет: разбор шрифтов остаётся на стороне того,
//! кому текст действительно нужен.
//!
//! Кнопки нарисованы поверх мира -- в кадре есть камера, и она не должна
//! уносить интерфейс с собой.
//!
//! Запуск: `cargo run --example ui-button`

use std::path::PathBuf;

use frglib::{
    Button, Camera, Color, Event, FrameClock, Image, Keycode, Object, Rect, Vec2, Window,
};
use sdl3::pixels::Color as SdlColor;

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/assets")
        .join(name)
}

/// Ошибка шрифта и ошибка окна — разные типы, а `main` обязан вернуть один.
#[derive(Debug)]
enum Error {
    Sdl(sdl3::Error),
    Font(sdl3::ttf::FontError),
    Frglib(frglib::Error),
}

impl From<sdl3::Error> for Error {
    fn from(e: sdl3::Error) -> Self {
        Error::Sdl(e)
    }
}

impl From<sdl3::ttf::FontError> for Error {
    fn from(e: sdl3::ttf::FontError) -> Self {
        Error::Font(e)
    }
}

impl From<frglib::Error> for Error {
    fn from(e: frglib::Error) -> Self {
        Error::Frglib(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Sdl(e) => write!(f, "SDL: {e}"),
            Error::Font(e) => write!(f, "шрифт: {e}"),
            Error::Frglib(e) => write!(f, "frglib: {e}"),
        }
    }
}

fn main() -> Result<(), Error> {
    let ttf = sdl3::ttf::init()?;
    let font = ttf.load_font(asset("font.otf"), 28.0)?;
    let white = SdlColor::RGBA(238, 242, 250, 255);

    let mut window = Window::builder("frglib ui — кнопки")
        .size(720, 480)
        .background(Color::rgb(22, 24, 32))
        .pixel_art(true)
        .build()?;

    // Подпись -- это просто картинка. Кнопке всё равно, чем она нарисована.
    let caption = |text: &str| -> Result<Image, Error> {
        Ok(Image::from_surface(font.render(text).solid(white)?))
    };

    let mut play = Button::new(Rect::new(60.0, 150.0, 200.0, 56.0));
    let mut sound = Button::new(Rect::new(60.0, 226.0, 200.0, 56.0));
    let mut quit = Button::new(Rect::new(60.0, 302.0, 200.0, 56.0)).with_colors(
        Color::rgb(70, 34, 40),
        Color::rgb(120, 48, 56),
        Color::rgb(180, 70, 78),
    );

    let play_label = caption("Играть")?;
    let sound_label = caption("Звук")?;
    let quit_label = caption("Выход")?;

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);
    let mut camera = Camera::new(Vec2::new(360.0, 240.0));
    let mut ticks = 0u32;
    let mut sound_on = true;
    let mut marker = Vec2::new(360.0, 240.0);

    loop {
        let dt = clock.dt();
        ticks += 1;

        let events = window.poll()?;

        for event in &events {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => {
                    if !pressed.contains(key) {
                        pressed.push(*key);
                    }
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != *key),
                _ => {}
            }
        }

        let held = |key| pressed.contains(&key);
        let mut direction = Vec2::ZERO;
        if held(Keycode::Left) || held(Keycode::A) {
            direction += Vec2::new(-1.0, 0.0);
        }
        if held(Keycode::Right) || held(Keycode::D) {
            direction += Vec2::new(1.0, 0.0);
        }
        if held(Keycode::Up) || held(Keycode::W) {
            direction += Vec2::new(0.0, -1.0);
        }
        if held(Keycode::Down) || held(Keycode::S) {
            direction += Vec2::new(0.0, 1.0);
        }

        // Камера едет за точкой, а кнопки всё равно остаются на экране --
        // ровно ради этого `Button::draw` и снимает камеру.
        marker += direction.normalized() * 160.0 * dt;
        camera.position += (marker - camera.position) * (4.0 * dt).min(1.0);

        // Клик по отпусканию: `update` возвращает true в том кадре, когда
        // клик состоялся, а не когда кнопку нажали.
        if quit.update(&events) {
            return Ok(());
        }
        if sound.update(&events) {
            sound_on = !sound_on;
        }
        if play.update(&events) {
            println!("поиграли: {ticks}");
        }

        let view = window.size();
        let (vw, vh) = (view.0 as f32, view.1 as f32);

        // Отладочная строка: в библиотеке шрифтов нет, поэтому её рисует тот
        // же SDL3_ttf. Считаем до кадра — внутри замыкания это был бы второй
        // изменяемый займ на тот же ресурс.
        let say = |text: &str| -> Result<Image, Error> {
            Ok(Image::from_surface(font.render(text).solid(white)?))
        };
        let note = say(if sound_on {
            "звук: да"
        } else {
            "звук: нет"
        })?;
        let line = say(&format!(
            "Играть: {}   Звук: {}   Выход: {}",
            name(play.state()),
            name(sound.state()),
            name(quit.state())
        ))?;

        window.frame_with(|d| {
            d.set_camera(camera);

            // Мир под интерфейсом: точка едет и уносит с собой камеру.
            let dot = Object::from_xywh(marker.x, marker.y, 40.0, 40.0);
            let dot_color = if sound_on {
                Color::rgb(70, 200, 150)
            } else {
                Color::rgb(90, 90, 100)
            };
            d.fill_object(&dot, dot_color)?;

            play.draw(d, &play_label)?;
            sound.draw(d, &sound_label)?;
            quit.draw(d, &quit_label)?;

            // Состояние видно и текстом. Подписи считаются до кадра: внутри
            // замыкания это был бы второй изменяемый займ на тот же ресурс.
            let (nw, nh) = (note.size().0 as f32, note.size().1 as f32);
            let (lw, lh) = (line.size().0 as f32, line.size().1 as f32);

            // Всё это -- интерфейс, значит координаты экрана.
            d.screen_space(|d| {
                d.fill_textured_object(&Object::from_xywh(vw / 2.0, 60.0, nw, nh), &note)?;
                d.fill_textured_object(&Object::from_xywh(vw / 2.0, vh - 50.0, lw, lh), &line)
            })
        })?;

        clock.tick();
    }
}

/// Как называется состояние по-русски, для отладочной строки.
fn name(state: frglib::ButtonState) -> &'static str {
    match state {
        frglib::ButtonState::Idle => "обычная",
        frglib::ButtonState::Hover => "наведение",
        frglib::ButtonState::Pressed => "нажата",
    }
}
