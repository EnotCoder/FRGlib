//! Компактная строка состояния в правом нижнем углу: FPS и зум.
//!
//! В HUD наверху остаются только время и слои, а эти два числа -- служебные,
//! поэтому им хватает отдельной плашки у противоположного края.

use frglib::{Anchor, Draw, Label, Vec2};

use super::hud::{HudTexts, FPS_LABEL, ZOOM_LABEL};
use super::{draw_panel, Ui};
use crate::consts::{STATS_BLOCK_GAP, STATS_GAP, STATS_MARGIN, STATS_PADDING};

impl Ui<'_> {
    /// Рисует FPS и текущий зум в правом нижнем углу.
    pub fn draw_stats(
        &self,
        d: &mut Draw,
        texts: &HudTexts,
        window: Vec2,
    ) -> Result<(), frglib::Error> {
        let fps_key = self.dim.get(FPS_LABEL);
        let fps_value = self.value.get(&texts.fps);
        let zoom_key = self.dim.get(ZOOM_LABEL);
        let zoom_value = self.value.get(&texts.zoom);

        let row_height = [
            fps_key.size().1,
            fps_value.size().1,
            zoom_key.size().1,
            zoom_value.size().1,
        ]
        .into_iter()
        .max()
        .unwrap_or(0) as f32;

        let content_width = fps_key.size().0 as f32
            + STATS_GAP
            + fps_value.size().0 as f32
            + STATS_BLOCK_GAP
            + zoom_key.size().0 as f32
            + STATS_GAP
            + zoom_value.size().0 as f32;

        let width = content_width + STATS_PADDING.x * 2.0;
        let height = row_height + STATS_PADDING.y * 2.0;
        let x = window.x - STATS_MARGIN.x - width;
        let y = window.y - STATS_MARGIN.y - height;
        let text_y = y + STATS_PADDING.y;

        d.screen_space(|d| {
            draw_panel(d, x, y, width, height, None)?;

            let mut cursor = x + STATS_PADDING.x;

            Label::new(fps_key)
                .at(cursor, text_y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;
            cursor += fps_key.size().0 as f32 + STATS_GAP;

            Label::new(fps_value)
                .at(cursor, text_y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;
            cursor += fps_value.size().0 as f32 + STATS_BLOCK_GAP;

            Label::new(zoom_key)
                .at(cursor, text_y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;
            cursor += zoom_key.size().0 as f32 + STATS_GAP;

            Label::new(zoom_value)
                .at(cursor, text_y)
                .anchored(Anchor::TopLeft)
                .draw(d)?;

            Ok(())
        })
    }
}
