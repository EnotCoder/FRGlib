//! Данные и карточка выбранного тела.

use std::error::Error;

use frglib::{Anchor, Color, Draw, Label};

use super::{draw_panel, draw_separator, Ui};
use crate::bodies::{Moon, Planet};
use crate::consts::{
    ACCENT_MOON, ACCENT_PLANET, ACCENT_STAR, INFO_FOOTER_GAP, INFO_PANEL_ORIGIN,
    INFO_PANEL_PADDING, INFO_PANEL_WIDTH, INFO_ROW_HEIGHT, INFO_SECTION_GAP, INFO_TITLE_GAP,
};

const FOOTER_IDLE: &str = "F follow    ESC close";
const FOOTER_FOLLOW: &str = "following    F unlock    ESC close";

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
    /// Тип тела: по нему выбирается цвет акцентной полосы.
    pub fn accent(&self) -> Color {
        match self.kind {
            "Star" => ACCENT_STAR,
            "Moon" => ACCENT_MOON,
            _ => ACCENT_PLANET,
        }
    }

    /// Подпись типа в правом верхнем углу карточки.
    pub fn kind(&self) -> &'static str {
        self.kind
    }

    /// Строки карточки: название характеристики и готовое значение.
    /// Форматирование одно и то же, поэтому заголовки выровнены по колонкам.
    pub fn rows(&self) -> Vec<(&'static str, String)> {
        let mut rows = Vec::with_capacity(4);
        rows.push(("Radius", format!("{} km", group_digits(self.radius_km))));

        if let Some(distance) = self.distance_km {
            let key = if self.kind == "Moon" {
                "Orbit"
            } else {
                "Distance"
            };
            rows.push((key, format_distance(distance)));
        }

        if let Some(period) = self.period_days {
            rows.push(("Period", format_period(period)));
        }

        if self.moon_count > 0 {
            rows.push(("Moons", self.moon_count.to_string()));
        }

        rows
    }
}

pub fn planet_info(planet: &Planet) -> BodyInfo {
    BodyInfo {
        name: planet.name,
        kind: "Planet",
        radius_km: planet.radius_km,
        distance_km: Some(planet.distance_km),
        period_days: Some(planet.year_days),
        moon_count: planet.moon_count,
    }
}

pub fn moon_info(moon: &Moon) -> BodyInfo {
    BodyInfo {
        name: moon.name,
        kind: "Moon",
        radius_km: moon.radius_km,
        distance_km: Some(moon.orbit_km),
        period_days: Some(moon.period_days),
        moon_count: 0,
    }
}

pub fn sun_info() -> BodyInfo {
    BodyInfo {
        name: "Sun",
        kind: "Star",
        radius_km: 696_340.0,
        distance_km: None,
        period_days: None,
        moon_count: 0,
    }
}

impl Ui<'_> {
    /// Готовит карточку выбранного тела к отрисовке в этом кадре.
    pub fn keep_info(&mut self, info: &BodyInfo) -> Result<(), Box<dyn Error>> {
        self.title.keep(info.name)?;
        self.dim.keep(info.kind())?;

        for (key, value) in info.rows() {
            self.dim.keep(key)?;
            self.value.keep(&value)?;
        }

        self.dim.keep(FOOTER_IDLE)?;
        self.dim.keep(FOOTER_FOLLOW)?;

        Ok(())
    }

    /// Панель с информацией о выбранном теле в левом верхнем углу.
    pub fn draw_info_card(
        &self,
        d: &mut Draw,
        info: &BodyInfo,
        following: bool,
    ) -> Result<(), frglib::Error> {
        let scale = self.scale;
        let rows = info.rows();
        let footer_text = if following {
            FOOTER_FOLLOW
        } else {
            FOOTER_IDLE
        };

        let title_height = self.title.get(info.name).size().1 as f32;
        let footer_height = self.dim.get(footer_text).size().1 as f32;

        let origin = self.size(INFO_PANEL_ORIGIN);
        let panel_width = self.px(INFO_PANEL_WIDTH);
        let x = origin.x;
        let y = origin.y;
        let padding = self.px(INFO_PANEL_PADDING);
        let content_x = x + padding;
        let content_right = x + panel_width - padding;

        let title_gap = self.px(INFO_TITLE_GAP);
        let section_gap = self.px(INFO_SECTION_GAP);
        let row_height = self.px(INFO_ROW_HEIGHT);
        let footer_gap = self.px(INFO_FOOTER_GAP);

        let title_y = y + padding;
        let separator_1 = title_y + title_height + title_gap;
        let rows_y = separator_1 + 1.0 + section_gap;
        let rows_bottom = rows_y + rows.len() as f32 * row_height;
        let separator_2 = rows_bottom + footer_gap;
        let footer_y = separator_2 + 1.0 + footer_gap;
        let height = footer_y + footer_height + padding - y;

        d.screen_space(|d| {
            draw_panel(d, x, y, panel_width, height, Some(info.accent()), scale)?;

            Label::new(self.title.get(info.name))
                .at(content_x, title_y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;

            // Тип тела прижат к нижнему краю заголовка -- так он читается
            // как подпись, а не как вторая строка названия.
            Label::new(self.dim.get(info.kind()))
                .at(content_right, title_y + title_height)
                .anchored(Anchor::BottomRight)
                .draw(d)?;

            draw_separator(d, content_x, separator_1, content_right - content_x, scale)?;

            for (index, (key, value)) in rows.iter().enumerate() {
                let row_y = rows_y + index as f32 * row_height;

                Label::new(self.dim.get(key))
                    .at(content_x, row_y)
                    .anchored(Anchor::TopLeft)
                    .draw(d)?;

                Label::new(self.value.get(value))
                    .at(content_right, row_y)
                    .anchored(Anchor::TopRight)
                    .draw(d)?;
            }

            draw_separator(d, content_x, separator_2, content_right - content_x, scale)?;

            Label::new(self.dim.get(footer_text))
                .at(content_x, footer_y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;

            Ok(())
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
