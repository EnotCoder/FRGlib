//! Интерфейс: карточка тела, HUD, подсказка управления и подписи.

mod controls;
mod hud;
mod info;
mod labels;

pub use hud::{HudState, HudTexts};
pub use info::{moon_info, planet_info, sun_info, BodyInfo};
pub use labels::collect_name_labels;

use std::error::Error;

use frglib::{Image, TextCache};

use crate::bodies::Planet;
use crate::consts::CONTROLS;

/// Как рисуется строка: шрифт и цвет зашиты в замыкание.
type Renderer<'a> = Box<dyn FnMut(&str) -> Result<Image, Box<dyn Error>> + 'a>;
type Cache<'a> = TextCache<Renderer<'a>>;

/// Владелец кэшей текста и всех панелей интерфейса.
pub struct Ui<'a> {
    info: Cache<'a>,
    hud: Cache<'a>,
}

impl<'a> Ui<'a> {
    /// Создаёт два кэша: крупный шрифт для карточки, мелкий для HUD.
    pub fn new(info_font: sdl3::ttf::Font<'a>, hud_font: sdl3::ttf::Font<'a>) -> Self {
        let info_renderer: Renderer<'a> = Box::new(move |text: &str| {
            let color = sdl3::pixels::Color::RGBA(232, 236, 244, 255);

            Ok(Image::from_surface(info_font.render(text).blended(color)?))
        });
        let hud_renderer: Renderer<'a> = Box::new(move |text: &str| {
            let color = sdl3::pixels::Color::RGBA(196, 205, 222, 255);

            Ok(Image::from_surface(hud_font.render(text).blended(color)?))
        });

        Self {
            info: TextCache::new(info_renderer),
            hud: TextCache::new(hud_renderer),
        }
    }

    /// Готовит карточку выбранного тела к отрисовке в этом кадре.
    pub fn keep_info(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        self.info.keep(text)
    }

    /// Готовит строки HUD. Вызывать до `frame_with`.
    pub fn keep_hud(&mut self, texts: &HudTexts) -> Result<(), Box<dyn Error>> {
        self.hud.keep(&texts.time)?;
        self.hud.keep(&texts.layers)?;
        self.hud.keep(&texts.fps)?;

        Ok(())
    }

    /// Имена тел не меняются -- кэшируем их один раз при запуске.
    pub fn keep_body_names(&mut self, planets: &[Planet]) -> Result<(), Box<dyn Error>> {
        for name in [
            "Sun", "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
        ] {
            self.hud.keep(name)?;
        }

        for planet in planets {
            for moon in &planet.moons {
                self.hud.keep(moon.name)?;
            }
        }

        Ok(())
    }

    /// Подсказка по управлению не меняется -- её тоже хватает один раз.
    pub fn keep_controls(&mut self) -> Result<(), Box<dyn Error>> {
        self.hud.keep(CONTROLS)
    }
}
