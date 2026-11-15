//! Строки HUD и панель в правом верхнем углу.

use frglib::{Anchor, Draw, Label, Object, Vec2};

use super::Ui;
use crate::consts::{
    FPS_BUCKET, HUD_FPS_Y, HUD_LAYERS_Y, HUD_PANEL_BG, HUD_PANEL_BORDER, HUD_PANEL_MARGIN,
    HUD_PANEL_SIZE, HUD_TEXT_X, HUD_TIME_Y,
};

/// Состояние, из которого собираются строки HUD.
pub struct HudState {
    pub paused: bool,
    pub speed: f32,
    pub show_orbits: bool,
    pub show_belts: bool,
    pub show_labels: bool,
    pub show_moons: bool,
    pub fps: f32,
}

/// Готовые строки кадра: их же кладём в кэш, их же рисуем.
pub struct HudTexts {
    pub time: String,
    pub layers: String,
    pub fps: String,
}

impl HudState {
    pub fn texts(&self) -> HudTexts {
        let time = if self.paused {
            "TIME   paused".to_string()
        } else {
            format!("TIME   x{}", self.speed)
        };
        let layers = format!(
            "orbits {}   belts {}\nlabels {}   moons {}",
            on_off(self.show_orbits),
            on_off(self.show_belts),
            on_off(self.show_labels),
            on_off(self.show_moons)
        );
        let fps = format!("FPS    {}", fps_bucket(self.fps));

        HudTexts { time, layers, fps }
    }
}

impl Ui<'_> {
    /// Рисует панель состояния в правом верхнем углу.
    pub fn draw_hud(
        &self,
        d: &mut Draw,
        texts: &HudTexts,
        window: Vec2,
    ) -> Result<(), frglib::Error> {
        let panel_x = window.x - HUD_PANEL_SIZE.x - HUD_PANEL_MARGIN.x;
        let panel_y = HUD_PANEL_MARGIN.y;

        let panel = Object::from_xywh(
            panel_x + HUD_PANEL_SIZE.x * 0.5,
            panel_y + HUD_PANEL_SIZE.y * 0.5,
            HUD_PANEL_SIZE.x,
            HUD_PANEL_SIZE.y,
        );

        d.screen_space(|d| {
            d.fill_object(&panel, HUD_PANEL_BG)?;
            d.draw_object(&panel, HUD_PANEL_BORDER)?;

            Label::new(self.hud.get(&texts.time))
                .at(panel_x + HUD_TEXT_X, panel_y + HUD_TIME_Y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;
            Label::new(self.hud.get(&texts.layers))
                .at(panel_x + HUD_TEXT_X, panel_y + HUD_LAYERS_Y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;
            Label::new(self.hud.get(&texts.fps))
                .at(panel_x + HUD_TEXT_X, panel_y + HUD_FPS_Y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;

            Ok(())
        })
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

fn fps_bucket(fps: f32) -> u32 {
    ((fps / FPS_BUCKET).round() * FPS_BUCKET) as u32
}
