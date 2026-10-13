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

use crate::camera::Camera;
use crate::color::Color;
use crate::math::Vec2;
use crate::rect::Rect;
use crate::window::Error;

use geometry::{polygon_vertices, project, to_f_rect};
use sdl3::pixels::FColor;
use sdl3::render::{BlendMode, FPoint, VertexIndices, WindowCanvas};

/// Draws shapes onto a window for one frame.
///
/// Obtained from [`Window::frame_with`](crate::Window::frame_with). Holds a
/// mutable borrow of the window, so it cannot outlive the frame it was handed
/// to.
///
/// Without a camera, coordinates are screen coordinates and go straight to the
/// renderer. [`set_camera`](Self::set_camera) switches the frame over to world
/// coordinates. A fresh `Draw` is made per frame, so one frame's camera can
/// never leak into the next.
pub struct Draw<'a> {
    canvas: &'a mut WindowCanvas,
    camera: Option<Camera>,
}

impl<'a> Draw<'a> {
    pub(crate) fn new(canvas: &'a mut WindowCanvas) -> Self {
        Draw {
            canvas,
            camera: None,
        }
    }

    /// Points the camera at `camera` for the rest of this frame.
    ///
    /// Takes it by value since [`Camera`] is `Copy`. Call it first, before any
    /// drawing: what earlier calls did is already on screen by then.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = Some(camera);
    }

    /// Window size in pixels, as the camera needs it.
    fn size_px(&self) -> (u32, u32) {
        self.canvas.window().size()
    }

    /// Places a world-space box onto the screen, or leaves it be when the frame
    /// has no camera.
    fn place_rect(&self, rect: Rect) -> Rect {
        match self.camera {
            Some(camera) => camera.project_rect(rect, self.size_px()),
            None => rect,
        }
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
    ///
    /// `rect` is in world coordinates; the camera places it.
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

        let rect: Rect = rect.into();
        let f_rect = to_f_rect(self.place_rect(rect));
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(Some(f_rect))?;
        Ok(())
    }

    /// Turns `points` about `obj`'s centre, in place of them as given.
    ///
    /// `points` are in world coordinates; the camera places them.
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

        let placed = project(points, self.camera, self.size_px());

        // Соединяем соседние точки.
        for pair in placed.windows(2) {
            let (x1, y1) = pair[0];
            let (x2, y2) = pair[1];

            self.canvas
                .draw_line(FPoint::new(x1, y1), FPoint::new(x2, y2))?;
        }

        // Замыкаем контур: последняя точка → первая.
        let (last_x, last_y) = placed[placed.len() - 1];
        let (first_x, first_y) = placed[0];

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

        let mut vertices = polygon_vertices(points, indices, vertex_color, false, None);
        if vertices.is_empty() {
            return Ok(());
        }

        // Built in world space, then placed. Without a camera the vertices are
        // already where they belong and the loop is skipped entirely.
        if let Some(camera) = self.camera {
            let size = self.size_px();
            for vertex in &mut vertices {
                let p = camera.to_screen(Vec2::new(vertex.position.x, vertex.position.y), size);
                vertex.position = FPoint::new(p.x, p.y);
            }
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
