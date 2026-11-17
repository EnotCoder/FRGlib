//! Пути к ассетам и загрузка Solar System.

use std::path::PathBuf;

use frglib::Image;

use crate::bodies::{get_planets, Planet};

/// Шрифт, общий для всех примеров.
pub fn font_path() -> PathBuf {
    shared_asset("font.otf")
}

/// Корень папки с ассетами.
///
/// На десктопе это `examples/2d_games/assets`, поэтому путь можно открыть
/// обычными средствами ОС. На Android `build.sh` кладёт ту же папку в APK,
/// и SDL открывает относительные пути через `AAssetManager`; абсолютный путь
/// хоста там не существует.
fn assets_root() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::new()
    }

    #[cfg(not(target_os = "android"))]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/2d_games/assets")
    }
}

/// Картинка из папки ассетов этого примера.
fn asset(name: &str) -> PathBuf {
    assets_root().join("space").join(name)
}

/// Ассет, общий для примеров: шрифт лежит один на всех.
fn shared_asset(name: &str) -> PathBuf {
    assets_root().join(name)
}

/// Загружает спрайт Солнца и собирает планеты с их спутниками.
pub fn load_solar_system() -> Result<(Image, Vec<Planet>), Box<dyn std::error::Error>> {
    let sun = Image::from_file(asset("sun.png"))?;

    let planets = [
        "mercury", "vinera", "earth", "mars", "jupiter", "saturn", "uran", "neptune",
    ]
    .iter()
    .map(|name| Image::from_file(asset(&format!("planets/{name}.png"))))
    .collect::<Result<Vec<_>, _>>()?;

    let moon_textures = [
        "moons/moon.png",
        "moons/thomas_deimos.png",
        "moons/io.png",
        "moons/callisto.png",
        "moons/ganymede.png",
        "moons/titan.png",
        "moons/rhea.png",
        "moons/titania.png",
        "moons/triton.png",
    ]
    .iter()
    .map(|name| Image::from_file(asset(name)).ok())
    .collect::<Vec<_>>();

    let mut planets = planets.into_iter();
    let mut next = || planets.next().expect("all planet sprites are loaded");

    Ok((
        sun,
        get_planets(
            next(),
            next(),
            next(),
            next(),
            next(),
            next(),
            next(),
            next(),
            &moon_textures,
        ),
    ))
}
