//! Показывает, как отслеживать состояние кнопок мыши между кадрами.
//!
//! Зажата левая — `fgoty_down_pose.png`, правая — `fgoty_up_pose.png`, обе
//! сразу или ни одной — `fgoty.png`.
//!
//! Запуск: `cargo run --example basic-mouse`

use std::path::PathBuf;
use std::time::Duration;

use frglib::{Color, Event, Image, MouseButton, Object, Window};

/// Картинки лежат в репозитории; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/basic/assets")
        .join(name)
}

fn main() -> Result<(), frglib::Error> {
    let neutral = Image::from_file(asset("fgoty.png"))?;
    let left_pose = Image::from_file(asset("fgoty_down_pose.png"))?;
    let right_pose = Image::from_file(asset("fgoty_up_pose.png"))?;

    let mut window = Window::builder("FRGLib mouse")
        .size(800, 600)
        .background(Color::rgb(20, 25, 30))
        .build()?;

    // Состояние живёт между кадрами: события его меняют, а очередной кадр
    // читает. Само по себе событие -- это «щёлкнули», а не «зажата сейчас».
    let mut left = false;
    let mut right = false;

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::MouseButtonDown { button, .. } => match button {
                    MouseButton::Left => left = true,
                    MouseButton::Right => right = true,
                    _ => {}
                },
                Event::MouseButtonUp { button, .. } => match button {
                    MouseButton::Left => left = false,
                    MouseButton::Right => right = false,
                    _ => {}
                },
                // При переключении на другое окно SDL не присылает
                // MouseButtonUp для зажатой кнопки, и флаг залип бы навсегда.
                Event::FocusLost => {
                    left = false;
                    right = false;
                }
                _ => {}
            }
        }

        let current = match (left, right) {
            (true, false) => &left_pose,
            (false, true) => &right_pose,
            // Ни одной или обе сразу -- нейтральная картинка.
            _ => &neutral,
        };

        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);

        // Вписываем картинку в окно целиком, с отступами, сохраняя пропорции.
        let margin = 20.0;
        let avail_w = (w - margin * 2.0).max(1.0);
        let avail_h = (h - margin * 2.0).max(1.0);
        let (iw, ih) = current.size();
        let ratio = iw as f32 / ih as f32;

        let (mut lw, mut lh) = (avail_w, avail_w / ratio);
        if lh > avail_h {
            lh = avail_h;
            lw = avail_h * ratio;
        }

        let x = (w - lw) / 2.0;
        let y = (h - lh) / 2.0;

        // Объект описывает картинку по центру, поэтому и рисуем её по центру.
        let sprite = Object::from_xywh(x + lw / 2.0, y + lh / 2.0, lw, lh);

        window.frame_with(|d| {
            d.fill_textured_object(&sprite, current)?;
            Ok(())
        })?;

        std::thread::sleep(Duration::from_millis(16));
    }
}
