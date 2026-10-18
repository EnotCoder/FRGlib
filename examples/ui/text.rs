//! Текст в окне: рисует SDL3_ttf, показывает frglib.
//!
//! Ни в библиотеке, ни в ассетах шрифт не нужен -- он лежит рядом, в
//! `assets/font.otf`, и грузится самим примером. Всё, что делает frglib, --
//! это [`Image::from_surface`]: обёртка поверхности, которую собрал кто-то
//! другой.
//!
//! Поэтому в `Cargo.toml` у этой библиотеки нет ни SDL3_ttf, ни `ab_glyph`, ни
//! `rusttype`. Тот, кому текст не нужен, не получает и тех двадцати шести
//! тысяч строк разбора шрифтов. А кому нужен -- подключает любой растеризатор
//! сам: тут SDL3_ttf, но `ab_glyph` или `rusttype` подошли бы так же, надо
//! только вернуть `Surface`.
//!
//! Запуск: `cargo run --example ui-text`

use std::collections::HashMap;
use std::path::PathBuf;

use frglib::{Camera, Color, Event, FrameClock, Image, Keycode, Object, Vec2, Window};
use sdl3::pixels::Color as SdlColor;
use sdl3::ttf::{Font, FontError};

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/assets")
        .join(name)
}

/// Кладёт строку в кеш, если её там ещё нет.
///
/// `Font::render` заново разбирает контуры и заливает поверхность на каждом
/// вызове, а «Очки: 120» меняется раз в секунду. Ключ — сама строка, так что
/// счётчик со ста значениями не даёт закешировать двести картинок.
///
/// Отдельная функция, а не «верни ссылку», потому что ссылок на один кеш за
/// кадр нужно несколько, а изменяемых займов из одного места — один.
fn keep(
    font: &Font,
    cache: &mut HashMap<String, Image>,
    text: &str,
    color: SdlColor,
) -> Result<(), Error> {
    if cache.contains_key(text) {
        return Ok(());
    }
    // `solid`, а не `blended`: сплошная заливка пиксель в пиксель, без
    // сглаживания. Для `pixel_art(true)` это ровно то, что нужно.
    let surface = font.render(text).solid(color)?;
    cache.insert(text.to_string(), Image::from_surface(surface));
    Ok(())
}

/// Наш собственный `Error` поверх двух чужих. `main` обязан вернуть один тип,
/// а ошибка шрифта и ошибка окна -- разные.
#[derive(Debug)]
enum Error {
    Sdl(sdl3::Error),
    Font(FontError),
    Frglib(frglib::Error),
}

impl From<sdl3::Error> for Error {
    fn from(e: sdl3::Error) -> Self {
        Error::Sdl(e)
    }
}

impl From<FontError> for Error {
    fn from(e: FontError) -> Self {
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

impl std::error::Error for Error {}

fn main() -> Result<(), Error> {
    // SDL3_ttf подключён только здесь: в библиотеке его нет.
    let ttf = sdl3::ttf::init()?;
    let font = ttf.load_font(asset("font.otf"), 48.0)?;

    let mut window = Window::builder("frglib ui — текст")
        .size(800, 480)
        .background(Color::rgb(24, 26, 34))
        .pixel_art(true)
        .build()?;

    let mut labels: HashMap<String, Image> = HashMap::new();

    let white = SdlColor::RGB(236, 240, 248);
    let dim = SdlColor::RGB(120, 132, 156);
    let warn = SdlColor::RGB(244, 180, 90);

    let mut pressed: Vec<Keycode> = Vec::new();
    let mut clock = FrameClock::new(60.0);
    let mut camera = Camera::new(Vec2::ZERO);
    let mut marker = Vec2::new(400.0, 240.0);
    let mut frame = 0u32;

    loop {
        let dt = clock.dt();
        frame += 1;

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => return Ok(()),
                Event::KeyDown { key, .. } => {
                    if !pressed.contains(&key) {
                        pressed.push(key);
                    }
                }
                Event::KeyUp { key, .. } => pressed.retain(|&held| held != key),
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
        marker += direction.normalized() * 220.0 * dt;

        // Камера едет за маркером, и текст, привязанный к миру, едет вместе с
        // ним. Убери `set_camera` -- и тот же текст окажется на экране.
        camera.position += (marker - camera.position) * (6.0 * dt).min(1.0);

        // Размер окна читаем до `frame_with`: замыкание и так берёт окно
        // на изменение, а внутри осталось бы обращаться к нему.
        let view = window.size();
        let (vw, vh) = (view.0 as f32, view.1 as f32);

        let (_, text_h) = font.size_of("Очки: 000")?;
        let text_h = text_h as f32;

        // Камера едет за маркером. Всё, что ниже, рисуется в мировых
        // координатах -- значит привязанный к миру текст уезжает вместе с
        // ним, а текст, привязанный к камере, стоит на месте.
        camera.position += (marker - camera.position) * (6.0 * dt).min(1.0);

        const MARKER: &str = "я маркер";
        const HINT: &str = "WASD -- гонять текст по миру";

        // Сначала всё кладём в кеш, потом берём оттуда: несколько ссылок на
        // один кеш за кадр -- нормально, несколько изменяемых займов -- нет.
        let score = format!("Очки: {:03}", frame / 30);
        keep(&font, &mut labels, MARKER, white)?;
        keep(&font, &mut labels, &score, warn)?;
        keep(&font, &mut labels, HINT, dim)?;

        let marker_label = &labels[MARKER];
        let score_img = &labels[score.as_str()];
        let hint_img = &labels[HINT];

        let marker_w = marker_label.size().0 as f32;
        let score_w = score_img.size().0 as f32;
        let hint_w = hint_img.size().0 as f32;

        window.frame_with(|d| {
            d.set_camera(camera);

            // Привязан к миру: едет за камерой вместе с маркером.
            d.fill_textured_object(
                &Object::from_xywh(marker.x, marker.y - 90.0, marker_w, text_h),
                marker_label,
            )?;

            // Привязан к камере: стоит на своём месте экрана, потому что
            // координаты берутся от неё самой.
            d.fill_textured_object(
                &Object::from_xywh(
                    camera.position.x,
                    camera.position.y - vh / 2.0 + 110.0,
                    score_w,
                    text_h,
                ),
                score_img,
            )?;

            d.fill_textured_object(
                &Object::from_xywh(
                    camera.position.x,
                    camera.position.y + vh / 2.0 - 110.0,
                    hint_w,
                    text_h,
                ),
                hint_img,
            )?;

            let _ = (vw, vh);
            Ok(())
        })?;

        clock.tick();
    }
}
