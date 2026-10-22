//! Анимация из кадров: восемь картинок, залитых на GPU один раз.
//!
//! Кадры лежат в `assets/rotate_rust` и нумеруются. На каждом кадре рисуется
//! ровно одна из уже залитых текстур -- ничего не заливается заново, поэтому
//! анимация не стоит ничего, пока её время не двигается.
//!
//! Управление: пробел -- пауза, `+`/`-` -- скорость, `R` -- сначала,
//! `L` -- разомкнуть или замкнуть цикл, Escape -- выход.
//!
//! Запуск: `cargo run --features cached-textures --example basic-animation`

use frglib::{Animation, Color, Event, FrameClock, Image, Keycode, Object, Window};
use std::path::PathBuf;

/// Кадры лежат в репозитории; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn frames_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/basic/assets/rotate_rust")
}

/// Каталог с кадрами, отсортированный по числу в имени.
///
/// Лексикографической сортировки здесь не хватить: `10.png` едет раньше
/// `2.png`. Поэтому из имени берётся число, а если числа нет -- берётся
/// полное имя, чтобы файл всё равно попал в список и его отсутствие было
/// видно, а не молча потерялось.
///
/// Ошибка здесь `io::Error`, а не [`frglib::Error`]: каталог читает
/// стандартная библиотека, и SDL к этому моменту ни при чём.
fn frame_files() -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(frames_dir())?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .collect();

    paths.sort_by_key(|path| {
        let number = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.parse::<u32>().ok());
        (number, path.clone())
    });

    Ok(paths)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = frame_files()?;

    // Все кадры заливаются на GPU до первого кадра. Это и есть вся разница
    // против рисования через `Image`: дальше игра тратит на анимацию
    // переключение индекса, а не переваливание мегабайт.
    let mut window = Window::builder("FRGLib animation")
        .size(800, 600)
        .background(Color::FON)
        .pixel_art(true)
        .build()?;

    let mut frames = Vec::with_capacity(paths.len());
    for path in &paths {
        let image = Image::from_file(path)?;
        frames.push(window.upload(&image)?);
    }

    println!("залито кадров: {}", frames.len());

    // Восемь кадров в секунду: кадры уходят по одному, и это видно. Выше --
    // уже смаз, и пиксель-арт перестаёт читаться.
    let mut animation = Animation::new(8.0).with_frames(frames);

    // Размер кадра одинаков у всех, но берётся у первого: список имеет право
    // меняться, а считать надо от того, что есть.
    let (iw, ih) = {
        let image = Image::from_file(&paths[0])?;
        image.size()
    };

    let mut paused = false;
    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);

    loop {
        // Реальное время кадра, а не константа: скорость анимации не должна
        // зависеть от того, сколько кадров в секунду тянет машина.
        let dt = clock.dt();

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),

                // Щелчок, а не «зажато»: удержание `+` не должно разогнать
                // анимацию до сотен кадров в секунду за один кадр.
                Event::KeyDown { key, .. } if !pressed.contains(&key) => {
                    pressed.push(key);
                    match key {
                        Keycode::Escape => return Ok(()),
                        Keycode::Space => {
                            paused = !paused;
                            println!("{}", if paused { "пауза" } else { "играет" });
                        }
                        Keycode::R => {
                            animation.rewind();
                            println!("сначала");
                        }
                        Keycode::L => {
                            // Цикл по умолчанию замкнут; `L` переключает.
                            let looped = animation.is_looped();
                            animation.set_looped(!looped);
                            println!(
                                "{}",
                                if looped {
                                    "цикл разомкнут"
                                } else {
                                    "цикл замкнут"
                                }
                            );
                        }
                        Keycode::Plus | Keycode::Equals | Keycode::KpPlus => {
                            animation.set_fps(animation.fps() + 2.0);
                            println!("{} к/с", animation.fps());
                        }
                        Keycode::Minus | Keycode::KpMinus => {
                            // Ноль означает заморозку, поэтому до 2 и не ниже.
                            animation.set_fps((animation.fps() - 2.0).max(2.0));
                            println!("{} к/с", animation.fps());
                        }
                        _ => {}
                    }
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != key),
                _ => {}
            }
        }

        // Пауза -- это просто «время не двигается». Сама анимация ничего про
        // паузу не знает, и это всё, что для неё нужно.
        if !paused {
            animation.advance(dt);
        }

        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);

        // Вписываем кадр в окно целиком, с отступом, сохраняя пропорции.
        let margin = 40.0;
        let avail_w = (w - margin * 2.0).max(1.0);
        let avail_h = (h - margin * 2.0).max(1.0);
        let ratio = iw as f32 / ih as f32;

        let (mut lw, mut lh) = (avail_w, avail_w / ratio);
        if lh > avail_h {
            lh = avail_h;
            lw = avail_h * ratio;
        }

        let sprite = Object::from_xywh(w / 2.0, h / 2.0, lw, lh);

        window.frame_with(|d| {
            d.fill_object_animation(&sprite, &animation)?;
            Ok(())
        })?;

        clock.tick();
    }
}
