//! Слайдеры: перетаскивание, диапазон и шаг.
//!
//! Слайдер, в отличие от кнопки, не знает своего текста: подпись стоит
//! *рядом* с дорожкой, а не на ней, иначе ручка наполовину её закрывает. Поэтому
//! `Slider::draw` рисует только дорожку, заливку и ручку, а текст -- тот же
//! SDL3_ttf, что и в примере с кнопками, посаженный на экран через [`Label`].
//!
//! Три слайдера подобраны так, чтобы показать три разных случая: непрерывный
//! (громкость), с дробным шагом (скорость) и целочисленный (размер). Ручка
//! при этом едет по-разному, и разницу видно.
//!
//! Слайдеры нарисованы поверх мира -- в кадре есть камера, и она не должна
//! уносить интерфейс с собой.
//!
//! Запуск: `cargo run --example ui-slider`

use std::path::PathBuf;

use frglib::{
    Anchor, Camera, Color, Event, FrameClock, Image, Keycode, Label, Object, Rect, Slider,
    SliderState, TextCache, Vec2, Window,
};
use sdl3::pixels::Color as SdlColor;

/// Размеры дорожки. Горизонтальное положение -- не константа: оно считается
/// от настоящего размера окна, см. `layout`.
const TRACK_W: f32 = 330.0;
const TRACK_H: f32 = 26.0;

/// Вертикальный шаг между строками, когда места хватает.
const ROW_STEP: f32 = 96.0;

/// Сколько строк дорожек.
const ROWS: usize = 3;

/// Сколько сверху занимает заголовок: блок дорожек не поднимается выше.
const HEADING_H: f32 = 96.0;

/// Сколько снизу отдано под подсказку и отладочную строку.
const FOOTER_H: f32 = 90.0;

/// Щель между краем дорожки и текстом рядом с ней.
const GAP: f32 = 20.0;

/// Отступ от края окна. Текст не должен липнуть к нему впритык.
const MARGIN: f32 = 8.0;

/// Сколько точек следа набирается на полной громкости.
const TRAIL: usize = 10;

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/ui/assets")
        .join(name)
}

/// Раскладка под текущий размер окна: три дорожки по центру.
///
/// Раньше тут стояли захардкоженные `x` и `y`, подсчитанные один раз при
/// старте. В tiled-режиме WM даёт окну совсем не те 720x480, что запрошены,
/// и весь блок с подписями уезжал за края. Теперь положение считается от
/// `view`, а при нехватке высоты шаг сжимается, чтобы дорожки оставались
/// в окне.
fn layout(view: (u32, u32)) -> [Rect; ROWS] {
    let (vw, vh) = (view.0 as f32, view.1 as f32);

    let x = ((vw - TRACK_W) / 2.0).max(0.0);

    // Вертикальная полоса, отданная дорожкам: от низа заголовка до верха
    // подсказки. Если она уже блока, шаг сжимается; дорожки при этом не
    // уменьшаются, иначе ручка стала бы неудобно мелкой.
    let band = (vh - HEADING_H - FOOTER_H - TRACK_H * ROWS as f32).max(0.0);
    let step = (band / (ROWS - 1) as f32).clamp(TRACK_H, ROW_STEP);
    let top = HEADING_H + (band - step * (ROWS - 1) as f32) / 2.0;

    std::array::from_fn(|row| Rect::new(x, top + row as f32 * step, TRACK_W, TRACK_H))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ttf = sdl3::ttf::init()?;
    // Два кегля: крупный для подписей и заголовка, мелкий для длинных строк
    // внизу. Один кегль на всё означал бы либо мелкий заголовок, либо строку,
    // которая не влезает в окно. Кеша тоже два -- по одному на кегль.
    let font = ttf.load_font(asset("font.otf"), 38.0)?;
    let font_small = ttf.load_font(asset("font.otf"), 24.0)?;
    let white = SdlColor::RGBA(238, 242, 250, 255);
    let dim = SdlColor::RGBA(140, 152, 180, 255);

    let mut window = Window::builder("frglib ui — слайдеры")
        .size(720, 480)
        .background(Color::rgb(22, 24, 32))
        .pixel_art(true)
        .build()?;

    // Раскладка живёт от размера окна, а WM в tiled-режиме отдаёт совсем не
    // те 720x480, что запрошены. Поэтому размер читается каждый кадр, а
    // слайдеры пересобираются -- но только когда он действительно изменился,
    // иначе сбрасывалось бы состояние перетаскивания.
    let mut view = window.size();
    let mut rects = layout(view);

    // Три случая: непрерывный, с дробным шагом и целочисленный.
    let mut volume = Slider::new(rects[0]).with_range(0.0, 1.0, 0.0);
    let mut speed = Slider::new(rects[1]).with_range(0.0, 3.0, 0.1).with_colors(
        Color::rgb(44, 38, 62),
        Color::rgb(150, 84, 196),
        Color::rgb(232, 220, 248),
        Color::rgb(196, 150, 60),
    );
    let mut size = Slider::new(rects[2])
        .with_range(8.0, 72.0, 8.0)
        .with_colors(
            Color::rgb(38, 46, 42),
            Color::rgb(64, 158, 120),
            Color::rgb(214, 244, 232),
            Color::rgb(196, 150, 60),
        );

    // Кеш рендерит сам: библиотека шрифтов не видит, ей достаётся готовая
    // картинка. Считаем до кадра -- внутри замыкания это был бы второй
    // изменяемый займ на тот же ресурс.
    let mut big = TextCache::new(
        |text: &str| -> Result<Image, Box<dyn std::error::Error>> {
            Ok(Image::from_surface(font.render(text).solid(white)?))
        },
    );
    let mut small = TextCache::new(
        |text: &str| -> Result<Image, Box<dyn std::error::Error>> {
            Ok(Image::from_surface(font_small.render(text).solid(dim)?))
        },
    );

    // Подписи и подсказка не меняются, поэтому кладутся в кеш один раз.
    big.keep("СЛАЙДЕРЫ")?;
    big.keep("громкость")?;
    big.keep("скорость")?;
    big.keep("размер")?;
    small.keep("тяните мышью: значение меняется уже на нажатии")?;

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);
    let mut camera = Camera::new(Vec2::new(360.0, 240.0));
    let mut marker = Vec2::new(360.0, 240.0);
    let mut trail: Vec<Vec2> = Vec::new();
    let mut moved = 0u32;

    loop {
        let dt = clock.dt();

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

        // WM мог отдать окну другой размер: в tiled-режиме это норма, а не
        // исключение. Раскладка пересобирается от настоящего размера, и
        // значения слайдеров переносятся -- иначе resize сбрасывал бы всё
        // в ноль.
        let now = window.size();
        if now != view {
            view = now;
            rects = layout(view);

            let (v, s, z) = (volume.value(), speed.value(), size.value());
            volume = Slider::new(rects[0]).with_range(0.0, 1.0, 0.0);
            volume.set_value(v);
            speed = Slider::new(rects[1]).with_range(0.0, 3.0, 0.1).with_colors(
                Color::rgb(44, 38, 62),
                Color::rgb(150, 84, 196),
                Color::rgb(232, 220, 248),
                Color::rgb(196, 150, 60),
            );
            speed.set_value(s);
            size = Slider::new(rects[2])
                .with_range(8.0, 72.0, 8.0)
                .with_colors(
                    Color::rgb(38, 46, 42),
                    Color::rgb(64, 158, 120),
                    Color::rgb(214, 244, 232),
                    Color::rgb(196, 150, 60),
                );
            size.set_value(z);
        }

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

        // `update` возвращает true в том кадре, когда значение поехало. Это
        // ровно то, что нужно, чтобы не печатать строку sixty раз в секунду
        // и не заводить звук на каждый кадр drag без движения.
        if volume.update(&events) || speed.update(&events) || size.update(&events) {
            moved += 1;
        }

        // Мир под интерфейсом. Второй слайдер гонит точку, третий задаёт ей
        // размер, первый задаёт сколько точек следа за ней тянется.
        let side = size.value();
        let step = 220.0 * speed.fraction();

        marker += direction.normalized() * step * dt;
        camera.position += (marker - camera.position) * (4.0 * dt).min(1.0);

        trail.insert(0, marker);
        while trail.len() > TRAIL {
            trail.pop();
        }

        // Значения и состояния -- строки, которые меняются, поэтому кладутся
        // в кеш каждый кадр: закешированное не перерисовывается.
        let value_volume = format!("{:.2}", volume.value());
        let value_speed = format!("{:.1}", speed.value());
        let value_size = format!("{:.0}", size.value());
        let states = format!(
            "громкость: {}   скорость: {}   размер: {}   изменений: {}",
            name(volume.state()),
            name(speed.state()),
            name(size.state()),
            moved
        );

        big.keep(&value_volume)?;
        big.keep(&value_speed)?;
        big.keep(&value_size)?;
        small.keep(&states)?;

        let (vw, vh) = (view.0 as f32, view.1 as f32);

        window.frame_with(|d| {
            d.set_camera(camera);

            // След: чем больше громкость, тем длиннее хвост.
            let tail = (volume.value() * TRAIL as f32) as usize;
            for (i, at) in trail.iter().take(tail).enumerate() {
                let fade = 1.0 - i as f32 / TRAIL as f32;
                d.fill_object(
                    &Object::from_xywh(at.x, at.y, side * fade, side * fade),
                    Color::rgba(72, 140, 210, (200.0 * fade) as u8),
                )?;
            }

            d.fill_object(
                &Object::from_xywh(marker.x, marker.y, side, side),
                Color::rgb(96, 200, 150),
            )?;

            // Интерфейс -- значит координаты экрана. Именно это и делает
            // `Slider::draw` и `Label::draw` внутри: они снимают камеру на
            // время вызова.
            volume.draw(d)?;
            speed.draw(d)?;
            size.draw(d)?;

            Label::new(big.get("СЛАЙДЕРЫ"))
                .anchored(Anchor::Top)
                .at(vw / 2.0, 30.0)
                .draw(d)?;

            // Подпись прижата к левому краю дорожки, значение -- к правому.
            // Раньше тут стояли `120.0` и `650.0`, подсчитанные под 720
            // пикселей ширины: в tiled-режиме, где окно уже, текст уезжал
            // за край. Теперь всё считается от дорожки, а отступ от края
            // окна держит текст на месте даже в очень узком окне.
            for (row, (name_text, value_text)) in [
                ("громкость", value_volume.as_str()),
                ("скорость", value_speed.as_str()),
                ("размер", value_size.as_str()),
            ]
            .into_iter()
            .enumerate()
            {
                let track = rects[row];
                let cy = track.y + track.h / 2.0;

                let name = Label::new(big.get(name_text)).anchored(Anchor::TopRight);
                let value = Label::new(big.get(value_text)).anchored(Anchor::TopLeft);

                // Якорь стоит ровно на краю дорожки, но так, чтобы сам текст
                // остался внутри окна: узкое окно сдвигает подпись вправо, а
                // значение влево, и они съезжаются на дорожку, но не за край.
                let name_x = (track.x - GAP).max(name.size().0 as f32 + MARGIN);
                let value_x =
                    (track.x + track.w + GAP).min(vw - value.size().0 as f32 - MARGIN);

                name.at(name_x, cy).draw(d)?;
                value.at(value_x, cy).draw(d)?;
            }

            Label::new(small.get("тяните мышью: значение меняется уже на нажатии"))
                .anchored(Anchor::Bottom)
                .at(vw / 2.0, vh - 66.0)
                .draw(d)?;

            Label::new(small.get(&states))
                .anchored(Anchor::Bottom)
                .at(vw / 2.0, vh - 26.0)
                .draw(d)
        })?;

        clock.tick();
    }
}

/// Как называется состояние по-русски, для отладочной строки.
fn name(state: SliderState) -> &'static str {
    match state {
        SliderState::Idle => "обычный",
        SliderState::Hover => "наведение",
        SliderState::Dragging => "тянется",
    }
}
