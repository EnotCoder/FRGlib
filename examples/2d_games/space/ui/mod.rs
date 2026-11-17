//! Интерфейс: карточка тела, HUD, подсказки и подписи.
//!
//! У каждой панели есть общий фон, рамка и акцентная полоса слева, поэтому
//! оформление живёт здесь, а раскладка -- в соседних модулях.

mod controls;
mod hud;
mod info;
mod labels;
mod stats;

pub use hud::HudState;
pub use info::{moon_info, planet_info, sun_info, BodyInfo};
pub use labels::collect_name_labels;

use std::error::Error;
use std::rc::Rc;

use frglib::{Color, Draw, Image, Object, TextCache, Vec2};

use crate::bodies::Planet;
use crate::consts::{
    PANEL_ACCENT_WIDTH, PANEL_BG, PANEL_BORDER, PANEL_SEPARATOR, TEXT_ACCENT, TEXT_BODY,
    TEXT_BRIGHT, TEXT_DIM, TEXT_GOOD, TEXT_SHADOW, TEXT_TITLE, TEXT_WARN,
};

/// Как рисуется строка: шрифт и цвет зашиты в замыкание.
type Renderer<'a> = Box<dyn FnMut(&str) -> Result<Image, Box<dyn Error>> + 'a>;
type Cache<'a> = TextCache<Renderer<'a>>;

/// Владелец кэшей текста и всех панелей интерфейса.
pub struct Ui<'a> {
    title: Cache<'a>,
    body: Cache<'a>,
    value: Cache<'a>,
    accent: Cache<'a>,
    good: Cache<'a>,
    warn: Cache<'a>,
    dim: Cache<'a>,
    shadow: Cache<'a>,
    /// Множитель для всех размеров интерфейса. Равен плотности пикселей окна:
    /// на телефоне и HiDPI-мониторе панели и отступы растут вместе со шрифтом.
    scale: f32,
}

impl<'a> Ui<'a> {
    /// Создаёт кэши текста: крупный шрифт для заголовка карточки и меньший
    /// для всего остального. Цвет входит в кэш, поэтому на каждый цвет --
    /// свой шрифт, но сам файл шрифта читается один раз.
    ///
    /// `scale` -- плотность пикселей окна; шрифты уже должны быть загружены
    /// с её учётом, а здесь она растягивает отступы и панели.
    pub fn new(info_font: sdl3::ttf::Font<'a>, hud_font: sdl3::ttf::Font<'a>, scale: f32) -> Self {
        let info_font = Rc::new(info_font);
        let hud_font = Rc::new(hud_font);

        let cache = |font: Rc<sdl3::ttf::Font<'a>>, color: Color| -> Cache<'a> {
            let color: sdl3::pixels::Color = color.into();

            TextCache::new(Box::new(move |text: &str| {
                Ok(Image::from_surface(font.render(text).blended(color)?))
            }))
        };

        Self {
            title: cache(Rc::clone(&info_font), TEXT_TITLE),
            body: cache(Rc::clone(&hud_font), TEXT_BODY),
            value: cache(Rc::clone(&hud_font), TEXT_BRIGHT),
            accent: cache(Rc::clone(&hud_font), TEXT_ACCENT),
            good: cache(Rc::clone(&hud_font), TEXT_GOOD),
            warn: cache(Rc::clone(&hud_font), TEXT_WARN),
            dim: cache(Rc::clone(&hud_font), TEXT_DIM),
            shadow: cache(Rc::clone(&hud_font), TEXT_SHADOW),
            scale: scale.max(1.0),
        }
    }

    /// Размер из `consts.rs` в пикселях экрана.
    pub(super) fn px(&self, value: f32) -> f32 {
        value * self.scale
    }

    /// То же для парных размеров: отступов, полей и размеров панелей.
    pub(super) fn size(&self, value: Vec2) -> Vec2 {
        value * self.scale
    }

    /// Имена тел не меняются -- кэшируем их один раз при запуске.
    /// Тень нужна, чтобы подпись читалась и на светлом диске планеты.
    pub fn keep_body_names(&mut self, planets: &[Planet]) -> Result<(), Box<dyn Error>> {
        for name in [
            "Sun", "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune",
        ] {
            self.value.keep(name)?;
            self.shadow.keep(name)?;
        }

        for planet in planets {
            for moon in &planet.moons {
                self.value.keep(moon.name)?;
                self.shadow.keep(moon.name)?;
            }
        }

        Ok(())
    }
}

/// Фон, рамка и акцентная полоса панели. Координаты -- экранные.
///
/// `scale` -- плотность пикселей: ширина акцентной полосы задана в тех же
/// условных единицах, что и остальная раскладка.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_panel(
    d: &mut Draw,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    accent: Option<Color>,
    scale: f32,
) -> Result<(), frglib::Error> {
    let panel = Object::from_xywh(x + width * 0.5, y + height * 0.5, width, height);

    d.fill_object(&panel, PANEL_BG)?;
    d.draw_object(&panel, PANEL_BORDER)?;

    if let Some(color) = accent {
        let bar_width = PANEL_ACCENT_WIDTH * scale;
        let bar = Object::from_xywh(x + bar_width * 0.5, y + height * 0.5, bar_width, height);

        d.fill_object(&bar, color)?;
    }

    Ok(())
}

/// Тонкая линия внутри панели.
pub(super) fn draw_separator(
    d: &mut Draw,
    x: f32,
    y: f32,
    width: f32,
    scale: f32,
) -> Result<(), frglib::Error> {
    d.fill_object(
        &Object::from_xywh(x + width * 0.5, y + 0.5, width, scale.max(1.0)),
        PANEL_SEPARATOR,
    )
}
