//! Drawing on a window.
//!
//! Three pieces, split by what they know:
//!
//! - [`geometry`] works in plain numbers -- vertex buffers, turning points --
//!   and never sees a canvas;
//! - the private methods here are the SDL-facing primitives every public
//!   method reduces to when an object has no rotation;
//! - [`object`] holds the public `*_object` API, which is what callers use.

mod geometry;
mod object;

use crate::color::Color;
use crate::rect::Rect;
use crate::window::Error;

use geometry::{polygon_vertices, to_f_rect};
use sdl3::pixels::FColor;
use sdl3::render::{BlendMode, FPoint, VertexIndices, WindowCanvas};

/// Draws shapes onto a window for one frame.
///
/// Obtained from [`Window::frame_with`](crate::Window::frame_with). Holds a
/// mutable borrow of the window, so it cannot outlive the frame it was handed
/// to.
pub struct Draw<'a> {
    canvas: &'a mut WindowCanvas,
}

impl<'a> Draw<'a> {
    pub(crate) fn new(canvas: &'a mut WindowCanvas) -> Self {
        Draw { canvas }
    }

    /// Fills a rectangle with `color`.
    ///
    /// Alpha is honoured: a colour with `a < 255` is blended over whatever is
    /// already there, one with `a == 255` replaces it. SDL defaults to
    /// `BlendMode::None`, which would ignore alpha entirely, so the mode is
    /// switched per call.
    ///
    /// Private because an axis-aligned box is just an [`Object`] with no
    /// rotation, and that is how callers reach this path.
    fn fill_rect(&mut self, rect: impl Into<Rect>, color: Color) -> Result<(), Error> {
        // Only touch the blend mode when it actually has to change; every call
        // is a GL state change otherwise.
        let want = if color.a < 255 {
            BlendMode::Blend
        } else {
            BlendMode::None
        };
        if self.canvas.blend_mode() != want {
            self.canvas.set_blend_mode(want);
        }

        let f_rect = to_f_rect(rect);
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(Some(f_rect))?;
        Ok(())
    }

    /// Turns `points` about `obj`'s centre, in place of them as given.
    fn draw_polygon(&mut self, points: &[(f32, f32)], color: Color) -> Result<(), Error> {
        // Контуру нужно минимум две точки: один отрезок и замыкающий.
        if points.len() < 2 {
            return Ok(());
        }

        let want = if color.a < 255 {
            BlendMode::Blend
        } else {
            BlendMode::None
        };
        if self.canvas.blend_mode() != want {
            self.canvas.set_blend_mode(want);
        }

        self.canvas.set_draw_color(color);

        // Соединяем соседние точки.
        for pair in points.windows(2) {
            let (x1, y1) = pair[0];
            let (x2, y2) = pair[1];

            self.canvas
                .draw_line(FPoint::new(x1, y1), FPoint::new(x2, y2))?;
        }

        // Замыкаем контур: последняя точка → первая.
        let (last_x, last_y) = points[points.len() - 1];
        let (first_x, first_y) = points[0];

        self.canvas
            .draw_line(FPoint::new(last_x, last_y), FPoint::new(first_x, first_y))?;

        Ok(())
    }

    /// Private because an indexed triangle list is what every `*_object` fill
    /// reduces to once rotation is in play.
    fn fill_polygon(
        &mut self,
        points: &[(f32, f32)],
        indices: &[i32],
        color: Color,
    ) -> Result<(), Error> {
        let vertex_color = FColor {
            r: color.r as f32 / 255.0,
            g: color.g as f32 / 255.0,
            b: color.b as f32 / 255.0,
            a: color.a as f32 / 255.0,
        };

        let vertices = polygon_vertices(points, indices, vertex_color, false, None);
        if vertices.is_empty() {
            return Ok(());
        }

        let want = if color.a < 255 {
            BlendMode::Blend
        } else {
            BlendMode::None
        };
        if self.canvas.blend_mode() != want {
            self.canvas.set_blend_mode(want);
        }

        self.canvas
            .render_geometry(&vertices, None, VertexIndices::Sequential)?;

        Ok(())
    }

    /// Drawable size of the window, as `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        self.canvas.window().size()
    }
}
