//! Drawing on top of a window.

use crate::color::Color;
use crate::rect::Rect;
use crate::window::Error;

use sdl3::pixels::FColor;
use sdl3::render::{FPoint, Vertex, VertexIndices, BlendMode, WindowCanvas};

/// `Option<FRect>: From<Rect>` does not exist, so the conversion goes through an
/// explicit local at every call site that needs one.
fn to_f_rect(rect: impl Into<Rect>) -> sdl3::render::FRect {
    let rect: Rect = rect.into();
    rect.into()
}

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

        let f_rect = to_f_rect(rect);
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

    /// Fills a rectangle with `image`, stretched to fit.
    ///
    /// The image is uploaded to the GPU on every call and dropped again straight
    /// after, so this costs a texture upload per frame. That is fine for the odd
    /// sprite, but a large image redrawn every frame will want a persistent
    /// texture instead.
    pub fn fill_texture(
        &mut self,
        rect: impl Into<Rect>,
        image: &crate::image::Image,
    ) -> Result<(), Error> {
        // `texture_creator` hands over ownership, so nothing here borrows
        // `self.canvas` -- the texture borrows the local creator instead, which
        // is what lets `copy` take `&mut self.canvas` below.
        let creator = self.canvas.texture_creator();
        let texture = creator.create_texture_from_surface(&image.surface)?;

        self.canvas.set_blend_mode(sdl3::render::BlendMode::Blend);
        self.canvas.copy(&texture, None, Some(to_f_rect(rect)))?;
        Ok(())
    }

    /// Draws `image` as a square with its top-left corner at `(x, y)`.
    pub fn fill_textured_square(
        &mut self,
        x: f32,
        y: f32,
        side: f32,
        image: &crate::image::Image,
    ) -> Result<(), Error> {
        self.fill_texture(Rect::square(x, y, side), image)
    }

    pub fn draw_polygon(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
    ) -> Result<(), Error> {

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

            self.canvas.draw_line(
                FPoint::new(x1, y1),
                FPoint::new(x2, y2),
            )?;
        }

        // Замыкаем контур: последняя точка → первая.
        let (last_x, last_y) = points[points.len() - 1];
        let (first_x, first_y) = points[0];

        self.canvas.draw_line(
            FPoint::new(last_x, last_y),
            FPoint::new(first_x, first_y),
        )?;

        Ok(())
    }

    pub fn fill_polygon(
        &mut self,
        points: &[(f32, f32)],
        indices: &[i32],
        color: Color,
    ) -> Result<(), Error> {
        assert!(
            indices.len() % 3 == 0,
            "Количество индексов должно быть кратно 3"
        );

        if indices.is_empty() {
            return Ok(());
        }

        let vertex_color = FColor {
            r: color.r as f32 / 255.0,
            g: color.g as f32 / 255.0,
            b: color.b as f32 / 255.0,
            a: color.a as f32 / 255.0,
        };

        let mut vertices = Vec::with_capacity(indices.len());

        for &index in indices {
            assert!(
                index >= 0 && (index as usize) < points.len(),
                "Индекс вершины {index} выходит за границы массива"
            );

            let (x, y) = points[index as usize];

            vertices.push(Vertex {
                position: FPoint::new(x, y),
                color: vertex_color,
                tex_coord: FPoint::new(0.0, 0.0),
            });

            let want = if color.a < 255 {
                BlendMode::Blend
            } else {
                BlendMode::None
            };

            if self.canvas.blend_mode() != want {
                self.canvas.set_blend_mode(want);
            }

            self.canvas.render_geometry(
                &vertices,
                None,
                VertexIndices::Sequential,
            )?;

            Ok(())
        }
    }

    /// Drawable size of the window, as `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        self.canvas.window().size()
    }
}
