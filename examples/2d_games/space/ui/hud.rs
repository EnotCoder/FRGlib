//! Строки HUD и панель в правом верхнем углу.

use std::error::Error;

use frglib::{Anchor, Draw, Label, Vec2};

use super::{draw_panel, draw_separator, Ui};
use crate::consts::{
    FPS_BUCKET, HUD_HEADER_Y, HUD_PANEL_MARGIN, HUD_PANEL_SIZE, HUD_ROW_HEIGHT, HUD_SEPARATOR_1_Y,
    HUD_SEPARATOR_2_Y, HUD_SPEED_Y, HUD_TEXT_X, HUD_TOGGLES_Y, PANEL_ACCENT,
};

pub(super) const HEADER: &str = "SYSTEM";
pub(super) const SPEED_LABEL: &str = "SPEED";
pub(super) const ON: &str = "on";
pub(super) const OFF: &str = "off";
pub(super) const RUNNING: &str = "RUNNING";
pub(super) const PAUSED: &str = "PAUSED";
pub(super) const FPS_LABEL: &str = "FPS";
pub(super) const ZOOM_LABEL: &str = "ZOOM";

const ORBITS_LABEL: &str = "ORBITS";
const BELTS_LABEL: &str = "BELTS";
const LABELS_LABEL: &str = "LABELS";
const MOONS_LABEL: &str = "MOONS";

/// Состояние, из которого собираются строки HUD.
pub struct HudState {
    pub paused: bool,
    pub speed: f32,
    pub zoom: f32,
    pub show_orbits: bool,
    pub show_belts: bool,
    pub show_labels: bool,
    pub show_moons: bool,
    pub fps: f32,
}

/// Готовые строки кадра: их же кладём в кэш, их же рисуем.
pub struct HudTexts {
    pub speed: String,
    pub zoom: String,
    pub fps: String,
    pub paused: bool,
    pub orbits: bool,
    pub belts: bool,
    pub labels: bool,
    pub moons: bool,
}

impl HudState {
    pub fn texts(&self) -> HudTexts {
        HudTexts {
            speed: format!("x{}", number(self.speed)),
            // Зум меняется плавно, поэтому показываем его шагами по 5%:
            // иначе кэш текста рос бы на каждом кадре прокрутки.
            zoom: format!("{}%", zoom_percent(self.zoom)),
            fps: fps_bucket(self.fps).to_string(),
            paused: self.paused,
            orbits: self.show_orbits,
            belts: self.show_belts,
            labels: self.show_labels,
            moons: self.show_moons,
        }
    }
}

impl Ui<'_> {
    /// Готовит строки HUD. Вызывать до `frame_with`.
    pub fn keep_hud(&mut self, texts: &HudTexts) -> Result<(), Box<dyn Error>> {
        self.value.keep(&texts.speed)?;
        self.value.keep(&texts.zoom)?;
        self.value.keep(&texts.fps)?;

        self.accent.keep(HEADER)?;
        self.dim.keep(SPEED_LABEL)?;
        self.dim.keep(FPS_LABEL)?;
        self.dim.keep(ZOOM_LABEL)?;
        self.good.keep(ON)?;
        self.dim.keep(OFF)?;
        self.good.keep(RUNNING)?;
        self.warn.keep(PAUSED)?;

        for label in [ORBITS_LABEL, BELTS_LABEL, LABELS_LABEL, MOONS_LABEL] {
            self.dim.keep(label)?;
        }

        Ok(())
    }

    /// Рисует панель состояния в правом верхнем углу.
    pub fn draw_hud(
        &self,
        d: &mut Draw,
        texts: &HudTexts,
        window: Vec2,
    ) -> Result<(), frglib::Error> {
        let scale = self.scale;
        let margin = self.size(HUD_PANEL_MARGIN);
        let panel = self.size(HUD_PANEL_SIZE);
        let text_x = self.px(HUD_TEXT_X);

        let width = panel.x;
        let height = panel.y;
        let x = window.x - width - margin.x;
        let y = margin.y;
        let right = x + width - text_x;

        let status = if texts.paused {
            self.warn.get(PAUSED)
        } else {
            self.good.get(RUNNING)
        };

        let toggles = [
            (ORBITS_LABEL, texts.orbits),
            (BELTS_LABEL, texts.belts),
            (LABELS_LABEL, texts.labels),
            (MOONS_LABEL, texts.moons),
        ];

        d.screen_space(|d| {
            draw_panel(d, x, y, width, height, Some(PANEL_ACCENT), scale)?;

            Label::new(self.accent.get(HEADER))
                .at(x + text_x, y + self.px(HUD_HEADER_Y))
                .anchored(Anchor::TopLeft)
                .draw(d)?;

            Label::new(status)
                .at(right, y + self.px(HUD_HEADER_Y))
                .anchored(Anchor::TopRight)
                .draw(d)?;

            draw_separator(
                d,
                x + text_x,
                y + self.px(HUD_SEPARATOR_1_Y),
                width - text_x * 2.0,
                scale,
            )?;

            Label::new(self.dim.get(SPEED_LABEL))
                .at(x + text_x, y + self.px(HUD_SPEED_Y))
                .anchored(Anchor::TopLeft)
                .draw(d)?;

            Label::new(self.value.get(&texts.speed))
                .at(right, y + self.px(HUD_SPEED_Y))
                .anchored(Anchor::TopRight)
                .draw(d)?;

            draw_separator(
                d,
                x + text_x,
                y + self.px(HUD_SEPARATOR_2_Y),
                width - text_x * 2.0,
                scale,
            )?;

            let row_height = self.px(HUD_ROW_HEIGHT);
            for (index, (label, enabled)) in toggles.iter().enumerate() {
                let row_y = y + self.px(HUD_TOGGLES_Y) + index as f32 * row_height;

                Label::new(self.dim.get(label))
                    .at(x + text_x, row_y)
                    .anchored(Anchor::TopLeft)
                    .draw(d)?;

                let state = if *enabled {
                    self.good.get(ON)
                } else {
                    self.dim.get(OFF)
                };

                Label::new(state)
                    .at(right, row_y)
                    .anchored(Anchor::TopRight)
                    .draw(d)?;
            }

            Ok(())
        })
    }
}

/// Число без лишних нулей: `1`, `0.25`, `12.5`.
fn number(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;

    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else if (rounded * 10.0).fract().abs() < 0.001 {
        format!("{rounded:.1}")
    } else {
        format!("{rounded:.2}")
    }
}

fn fps_bucket(fps: f32) -> u32 {
    let fps = fps.clamp(0.0, 999.0);

    ((fps / FPS_BUCKET).round() * FPS_BUCKET) as u32
}

/// Зум в процентах, округлённый до 5%: меньше вариантов -- меньше кэш.
fn zoom_percent(zoom: f32) -> u32 {
    ((zoom * 100.0 / 5.0).round() * 5.0) as u32
}
