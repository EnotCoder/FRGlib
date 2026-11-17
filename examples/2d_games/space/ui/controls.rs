//! Подсказка по управлению в левом нижнем углу.
//!
//! По умолчанию видна одна строка самых нужных клавиш. `H` раскрывает
//! полный список с секциями -- он иначе занимал бы пол-экрана постоянно.

use std::error::Error;

use frglib::{Anchor, Draw, Label, Vec2};

use super::{draw_panel, draw_separator, Ui};
use crate::consts::{
    CONTROLS, CONTROLS_MARGIN, CONTROLS_PADDING, HELP_DESC_X, HELP_KEY_X, HELP_PANEL_PADDING,
    HELP_PANEL_WIDTH, HELP_ROW_HEIGHT, HELP_SECTIONS, HELP_SECTION_GAP, HELP_SECTION_HEIGHT,
    HELP_TITLE, HELP_TITLE_GAP, PANEL_ACCENT,
};

impl Ui<'_> {
    /// Тексты подсказки не меняются -- кэшируем их один раз при запуске.
    pub fn keep_controls(&mut self) -> Result<(), Box<dyn Error>> {
        self.dim.keep(CONTROLS)?;
        self.accent.keep(HELP_TITLE)?;

        for &(section, rows) in HELP_SECTIONS {
            self.accent.keep(section)?;

            for &(key, description) in rows {
                self.value.keep(key)?;
                self.body.keep(description)?;
            }
        }

        Ok(())
    }

    /// Рисует либо свёрнутую подсказку, либо полный список по `H`.
    pub fn draw_controls(
        &self,
        d: &mut Draw,
        window: Vec2,
        help_open: bool,
    ) -> Result<(), frglib::Error> {
        if help_open {
            self.draw_help(d, window)
        } else {
            self.draw_hint(d, window)
        }
    }

    fn draw_hint(&self, d: &mut Draw, window: Vec2) -> Result<(), frglib::Error> {
        let scale = self.scale;
        let image = self.dim.get(CONTROLS);
        let padding = self.size(CONTROLS_PADDING);
        let margin = self.size(CONTROLS_MARGIN);
        let width = image.size().0 as f32 + padding.x * 2.0;
        let height = image.size().1 as f32 + padding.y * 2.0;
        let x = margin.x;
        let y = window.y - margin.y - height;

        d.screen_space(|d| {
            draw_panel(d, x, y, width, height, None, scale)?;

            Label::new(image)
                .at(x + padding.x, y + padding.y)
                .anchored(Anchor::TopLeft)
                .draw(d)
        })
    }

    fn draw_help(&self, d: &mut Draw, window: Vec2) -> Result<(), frglib::Error> {
        let scale = self.scale;
        let padding = self.px(HELP_PANEL_PADDING);
        let panel_width = self.px(HELP_PANEL_WIDTH);
        let title_gap = self.px(HELP_TITLE_GAP);
        let section_height = self.px(HELP_SECTION_HEIGHT);
        let section_gap = self.px(HELP_SECTION_GAP);
        let row_height = self.px(HELP_ROW_HEIGHT);

        let title_height = self.accent.get(HELP_TITLE).size().1 as f32;
        let content_width = panel_width - padding * 2.0;

        let mut height = padding * 2.0 + title_height + title_gap + 1.0 + title_gap;

        for &(_, rows) in HELP_SECTIONS {
            height += section_height + rows.len() as f32 * row_height + section_gap;
        }

        // Последний отступ снизу уже даёт поле панели.
        height -= section_gap;

        let margin = self.size(CONTROLS_MARGIN);
        let x = margin.x;
        let y = window.y - margin.y - height;
        let content_x = x + padding;

        d.screen_space(|d| {
            draw_panel(d, x, y, panel_width, height, Some(PANEL_ACCENT), scale)?;

            let mut cursor = y + padding;

            Label::new(self.accent.get(HELP_TITLE))
                .at(content_x, cursor)
                .anchored(Anchor::TopLeft)
                .draw(d)?;

            cursor += title_height + title_gap;
            draw_separator(d, content_x, cursor, content_width, scale)?;
            cursor += 1.0 + title_gap;

            for &(section, rows) in HELP_SECTIONS {
                Label::new(self.accent.get(section))
                    .at(content_x, cursor)
                    .anchored(Anchor::TopLeft)
                    .draw(d)?;

                cursor += section_height;

                for &(key, description) in rows {
                    Label::new(self.value.get(key))
                        .at(content_x + self.px(HELP_KEY_X), cursor)
                        .anchored(Anchor::TopLeft)
                        .draw(d)?;

                    Label::new(self.body.get(description))
                        .at(content_x + self.px(HELP_DESC_X), cursor)
                        .anchored(Anchor::TopLeft)
                        .draw(d)?;

                    cursor += row_height;
                }

                cursor += section_gap;
            }

            Ok(())
        })
    }
}
