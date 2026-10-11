//! Рисует логотип библиотеки в окне.
//!
//! Запуск: `cargo run --example logo`

use std::path::PathBuf;
use std::time::Duration;

use frglib::{Color, Event, Image, Rect, Window};

/// Логотип лежит в репозитории; путь строим от корня крейта, а не от
/// текущего каталога, чтобы `cargo run --example` работал из любого места.
fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(name)
}

fn main() -> Result<(), frglib::Error> {
    let logo = Image::from_file(asset("logo.png"))?;
    println!("логотип {:?}", logo.size());

    let mut window = Window::builder("FRGLib logo")
        .size(900, 500)
        .background(Color::rgb(16, 43, 38))
        .build()?;

    // Логотип 1760x640, держим его пропорции при любом размере окна.
    let margin = 60.0;
    let (lw, lh) = logo.size();
    let ratio = lw as f32 / lh as f32;

    loop {
        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => println!("key {key:?}"),
                _ => {}
            }
        }

        let (w, h) = window.size();
        let (w, h) = (w as f32, h as f32);

        // Вписываем логотип в окно целиком, с отступами.
        let avail_w = (w - margin * 2.0).max(1.0);
        let avail_h = (h - margin * 2.0).max(1.0);
        let (mut lw, mut lh) = (avail_w, avail_w / ratio);
        if lh > avail_h {
            lh = avail_h;
            lw = avail_h * ratio;
        }

        let x = (w - lw) / 2.0;
        let y = (h - lh) / 2.0;

        window.frame_with(|d| {
            d.fill_texture(Rect::new(x, y, lw, lh), &logo)?;
            Ok(())
        })?;

        std::thread::sleep(Duration::from_millis(16));
    }
}
