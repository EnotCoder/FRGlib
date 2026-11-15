//! Подсказка по управлению в левом нижнем углу.

use frglib::{Anchor, Draw, Label, Object, Vec2};

use super::Ui;
use crate::consts::{CONTROLS, CONTROLS_BG, CONTROLS_MARGIN, CONTROLS_PADDING};

impl Ui<'_> {
    /// Рисует плашку с горячими клавишами.
    pub fn draw_controls(&self, d: &mut Draw, window: Vec2) -> Result<(), frglib::Error> {
        let label = Label::new(self.hud.get(CONTROLS))
            .at(CONTROLS_MARGIN.x, window.y - CONTROLS_MARGIN.y)
            .anchored(Anchor::BottomLeft);
        let rect = label.rect();
        let panel = Object::from_xywh(
            rect.x + rect.w / 2.0 - CONTROLS_PADDING.x,
            rect.y + rect.h / 2.0 - CONTROLS_PADDING.y,
            rect.w + CONTROLS_PADDING.x * 2.0,
            rect.h + CONTROLS_PADDING.y * 2.0,
        );

        d.screen_space(|d| {
            d.fill_object(&panel, CONTROLS_BG)?;
            label.draw(d)
        })
    }
}
