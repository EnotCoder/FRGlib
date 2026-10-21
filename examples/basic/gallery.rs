//! Листалка картинок: на кадре рисуется одна, выбранная по индексу.
//!
//! ← или A — предыдущая, → или D — следующая, Escape — выход. Круговой
//! проход: после последней идёт первая. Имя и номер текущей картинки
//! печатаются в консоль.
//!
//! Запуск: `cargo run --example basic-gallery`

use frglib::{Color, Event, FrameClock, Image, Keycode, Object, Window};
use std::path::PathBuf;

/// Картинки лежат в репозитории; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/basic/assets")
        .join(name)
}

fn main() -> Result<(), frglib::Error> {
    // Имена и картинки идут вместе, чтобы номер в списке совпадал с номером
    // в векторе: разведённые по разным массивам, они разъехались бы при
    // первой же правке списка.
    let names = [
        "fgoty.png",
        "fgoty_down_pose.png",
        "fgoty_up_pose.png",
        "fgoty_with_body.png",
        "fgoty_park.png",
        "fgoty_sad.png",
        "run_fgoty.png",
        "rust_down_pose.png",
    ];

    let images: Vec<Image> = names
        .iter()
        .map(|name| Image::from_file(asset(name)))
        .collect::<Result<_, frglib::Error>>()?;

    let mut window = Window::builder("FRGLib gallery")
        .size(800, 600)
        .background(Color::FON)
        .build()?;

    let mut index = 0usize;
    println!("[1/{}] {}", images.len(), names[index]);

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),

                // Щелчок, а не «зажато»: листать быстрым удержанием того же
                // кадра нельзя, дребезг клавиши гасим повторным нажатием.
                Event::KeyDown { key, .. } if !pressed.contains(&key) => {
                    pressed.push(key);
                    match key {
                        Keycode::Left | Keycode::A => {
                            index = (index + images.len() - 1) % images.len()
                        }
                        Keycode::Right | Keycode::D => index = (index + 1) % images.len(),
                        Keycode::Escape => return Ok(()),
                        _ => {}
                    }
                    println!("[{}/{}] {}", index + 1, images.len(), names[index]);
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != key),
                _ => {}
            }
        }

        let current = &images[index];

        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);

        // Вписываем картинку в окно целиком, с отступом, сохраняя пропорции.
        // Все картинки списка одинакового размера, но считаем на каждой --
        // список имеет право меняться.
        let margin = 40.0;
        let avail_w = (w - margin * 2.0).max(1.0);
        let avail_h = (h - margin * 2.0).max(1.0);
        let (iw, ih) = current.size();
        let ratio = iw as f32 / ih as f32;

        let (mut lw, mut lh) = (avail_w, avail_w / ratio);
        if lh > avail_h {
            lh = avail_h;
            lw = avail_h * ratio;
        }

        // Центр окна известен напрямую, поэтому ставим объект туда и не
        // считаем левый верхний угол, как делают примеры с одной картинкой.
        let sprite = Object::from_xywh(w / 2.0, h / 2.0, lw, lh);

        window.frame_with(|d| {
            d.fill_textured_object(&sprite, current)?;
            Ok(())
        })?;

        clock.tick();
    }
}
