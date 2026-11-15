//! Данные и карточка выбранного тела.

use frglib::{Anchor, Draw, Label, Object};

use super::Ui;
use crate::bodies::{Moon, Planet};
use crate::consts::{INFO_PANEL_BG, INFO_PANEL_BORDER, INFO_PANEL_ORIGIN, INFO_PANEL_PADDING};

/// То, что показываем в панели после клика по телу.
#[derive(Clone, Copy)]
pub struct BodyInfo {
    pub name: &'static str,
    kind: &'static str,
    radius_km: f32,
    distance_km: Option<f32>,
    period_days: Option<f32>,
    moon_count: u32,
}

impl BodyInfo {
    /// Текст карточки. Форматируется один раз и кладётся в кэш.
    pub fn text(&self) -> String {
        let mut text = format!(
            "{}\n{}\nRadius: {} km",
            self.name,
            self.kind,
            group_digits(self.radius_km)
        );

        if let Some(distance) = self.distance_km {
            text.push_str(&format!("\nDistance: {}", format_distance(distance)));
        }

        if let Some(period) = self.period_days {
            text.push_str(&format!("\nPeriod: {}", format_period(period)));
        }

        if self.moon_count > 0 {
            text.push_str(&format!("\nMoons: {}", self.moon_count));
        }

        text
    }
}

pub fn planet_info(planet: &Planet) -> BodyInfo {
    BodyInfo {
        name: planet.name,
        kind: "planet",
        radius_km: planet.radius_km,
        distance_km: Some(planet.distance_km),
        period_days: Some(planet.year_days),
        moon_count: planet.moon_count,
    }
}

pub fn moon_info(moon: &Moon) -> BodyInfo {
    BodyInfo {
        name: moon.name,
        kind: "moon",
        radius_km: moon.radius_km,
        distance_km: Some(moon.orbit_km),
        period_days: Some(moon.period_days),
        moon_count: 0,
    }
}

pub fn sun_info() -> BodyInfo {
    BodyInfo {
        name: "Sun",
        kind: "star",
        radius_km: 696_340.0,
        distance_km: None,
        period_days: None,
        moon_count: 0,
    }
}

impl Ui<'_> {
    /// Панель с информацией о выбранном теле в левом верхнем углу.
    pub fn draw_info_card(&self, d: &mut Draw, text: &str) -> Result<(), frglib::Error> {
        let label = Label::new(self.info.get(text))
            .at(INFO_PANEL_ORIGIN.x, INFO_PANEL_ORIGIN.y)
            .anchored(Anchor::TopLeft);
        let rect = label.rect();
        let panel = Object::from_xywh(
            rect.x + rect.w / 2.0,
            rect.y + rect.h / 2.0,
            rect.w + INFO_PANEL_PADDING * 2.0,
            rect.h + INFO_PANEL_PADDING * 2.0,
        );

        d.screen_space(|d| {
            d.fill_object(&panel, INFO_PANEL_BG)?;
            d.draw_object(&panel, INFO_PANEL_BORDER)?;
            label.draw(d)
        })
    }
}

/// `1234567` -> `1 234 567`: так большие километры читаются.
fn group_digits(value: f32) -> String {
    let digits = value.abs().round().max(0.0).to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(' ');
        }

        out.push(ch);
    }

    out
}

fn format_distance(km: f32) -> String {
    if km >= 1_000_000.0 {
        format!("{:.1} mln km", km / 1_000_000.0)
    } else {
        format!("{} km", group_digits(km))
    }
}

fn format_period(days: f32) -> String {
    if days >= 365.0 {
        format!("{:.1} years", days / 365.25)
    } else {
        format!("{:.1} days", days)
    }
}
