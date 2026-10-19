//! Текст в окне: рисует SDL3_ttf, показывает frglib.
//!
//! Ни в библиотеке, ни в ассетах шрифт не нужен -- он лежит рядом, в
//! `assets/font.otf`, и грузится самим примером. Всё, что делает frglib, --
//! это [`Image::from_surface`]: обёртка поверхности, которую собрал кто-то
//! другой. А [`Label`] просто ставит готовую картинку на экран, поэтому не
//! приходится каждый раз руками распаковывать размер и собирать `Object`.
//!
//! Поэтому в `Cargo.toml` у этой библиотеки нет ни SDL3_ttf, ни `ab_glyph`, ни
//! `rusttype`. Тот, кому текст не нужен, не получает и тех двадцати шести
//! тысяч строк разбора шрифтов. А кому нужен -- подключает любой растеризатор
//! сам: тут SDL3_ttf, но `ab_glyph` или `rusttype` подошли бы так же, надо
//! только вернуть `Surface`.
//!
//! Запуск: `cargo run --example ui-text`

use std::path::PathBuf;

use frglib::{Color, FrameClock, Image, Label, Window};

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/ui/assets")
        .join(name)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SDL3_ttf подключён только здесь: в библиотеке его нет.
    let ttf = sdl3::ttf::init()?;
    let font = ttf.load_font(asset("font.otf"), 48.0)?;

    // Текст собирается в поверхность один раз: строка не меняется, а заново
    // разбирать контуры шрифта каждый кадр незачем.
    let hello = Image::from_surface(font.render("Hello, world!").solid((236, 240, 248, 255))?);

    let mut window = Window::builder("frglib ui — текст")
        .size(720, 320)
        .resizable(true)
        .background(Color::rgb(24, 26, 34))
        .pixel_art(true)
        .build()?;

    let mut clock = FrameClock::new(60.0);

    loop {
        for event in window.poll()? {
            if event.should_close() {
                return Ok(());
            }
        }

        // Центр считается от настоящего размера окна каждый кадр: WM в
        // tiled-режиме отдаёт окну не те 720x320, что запрошены, и текст,
        // привязанный к стартовому размеру, уехал бы за край.
        let (vw, vh) = window.size();
        let label = Label::new(&hello).at(vw as f32 / 2.0, vh as f32 / 2.0);

        window.frame_with(|d| label.draw(d))?;

        clock.tick();
    }
}
