//! Drawing on top of a window.

use crate::color::Color;
use crate::rect::Rect;
use crate::window::Error;

use sdl3::render::{BlendMode, WindowCanvas};

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
    pub fn fill_rect(&mut self, rect: impl Into<Rect>, color: Color) -> Result<(), Error> {
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

        let rect: Rect = rect.into();
        let f_rect: sdl3::render::FRect = rect.into();
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(Some(f_rect))?;
        Ok(())
    }

    /// Fills a square with `color`, its top-left corner at `(x, y)`.
    pub fn fill_square(&mut self, x: f32, y: f32, side: f32, color: Color) -> Result<(), Error> {
        self.fill_rect(Rect::square(x, y, side), color)
    }

    /// Fills a square with `color`, centred on `(cx, cy)`.
    pub fn fill_square_centered(
        &mut self,
        cx: f32,
        cy: f32,
        side: f32,
        color: Color,
    ) -> Result<(), Error> {
        self.fill_rect(Rect::square_centered(cx, cy, side), color)
    }

    /// Drawable size of the window, as `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        self.canvas.window().size()
    }
}
