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
        let image = self.dim.get(CONTROLS);
        let width = image.size().0 as f32 + CONTROLS_PADDING.x * 2.0;
        let height = image.size().1 as f32 + CONTROLS_PADDING.y * 2.0;
        let x = CONTROLS_MARGIN.x;
        let y = window.y - CONTROLS_MARGIN.y - height;

        d.screen_space(|d| {
            draw_panel(d, x, y, width, height, None)?;

            Label::new(image)
                .at(x + CONTROLS_PADDING.x, y + CONTROLS_PADDING.y)
                .anchored(Anchor::TopLeft)
                .draw(d)
        })
    }

    fn draw_help(&self, d: &mut Draw, window: Vec2) -> Result<(), frglib::Error> {
        let title_height = self.accent.get(HELP_TITLE).size().1 as f32;
        let content_width = HELP_PANEL_WIDTH - HELP_PANEL_PADDING * 2.0;

        let mut height =
            HELP_PANEL_PADDING * 2.0 + title_height + HELP_TITLE_GAP + 1.0 + HELP_TITLE_GAP;

        for &(_, rows) in HELP_SECTIONS {
            height += HELP_SECTION_HEIGHT + rows.len() as f32 * HELP_ROW_HEIGHT + HELP_SECTION_GAP;
        }

        // Последний отступ снизу уже даёт поле панели.
        height -= HELP_SECTION_GAP;

        let x = CONTROLS_MARGIN.x;
        let y = window.y - CONTROLS_MARGIN.y - height;
        let content_x = x + HELP_PANEL_PADDING;

        d.screen_space(|d| {
            draw_panel(d, x, y, HELP_PANEL_WIDTH, height, Some(PANEL_ACCENT))?;

            let mut cursor = y + HELP_PANEL_PADDING;

            Label::new(self.accent.get(HELP_TITLE))
                .at(content_x, cursor)
                .anchored(Anchor::TopLeft)
                .draw(d)?;

            cursor += title_height + HELP_TITLE_GAP;
            draw_separator(d, content_x, cursor, content_width)?;
            cursor += 1.0 + HELP_TITLE_GAP;

            for &(section, rows) in HELP_SECTIONS {
                Label::new(self.accent.get(section))
                    .at(content_x, cursor)
                    .anchored(Anchor::TopLeft)
                    .draw(d)?;

                cursor += HELP_SECTION_HEIGHT;

                for &(key, description) in rows {
                    Label::new(self.value.get(key))
                        .at(content_x + HELP_KEY_X, cursor)
                        .anchored(Anchor::TopLeft)
                        .draw(d)?;

                    Label::new(self.body.get(description))
                        .at(content_x + HELP_DESC_X, cursor)
                        .anchored(Anchor::TopLeft)
                        .draw(d)?;

                    cursor += HELP_ROW_HEIGHT;
                }

                cursor += HELP_SECTION_GAP;
            }

            Ok(())
        })
    }
}
