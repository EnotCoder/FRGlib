//! Drawing on top of a window.

use crate::color::Color;
use crate::math::Vec2;
use crate::object::Object;
use crate::rect::Rect;
use crate::window::Error;

use sdl3::pixels::FColor;
use sdl3::render::{BlendMode, FPoint, Vertex, VertexIndices, WindowCanvas};

/// `Option<FRect>: From<Rect>` does not exist, so the conversion goes through an
/// explicit local at every call site that needs one.
fn to_f_rect(rect: impl Into<Rect>) -> sdl3::render::FRect {
    let rect: Rect = rect.into();
    rect.into()
}

/// Two triangles over four corners, in the winding `polygon_vertices` expects.
const QUAD_INDICES: [i32; 6] = [0, 1, 2, 0, 2, 3];

/// Turns `points` about `obj`'s centre, as `to_screen_points` needs them.
fn rotate_points(points: &[(f32, f32)], obj: &Object) -> Vec<(f32, f32)> {
    points
        .iter()
        .map(|&(x, y)| {
            let turned = obj.turn(Vec2::new(x, y));
            (turned.x, turned.y)
        })
        .collect()
}

/// Builds the vertex buffer for an indexed triangle list.
///
/// Shared by [`Draw::fill_polygon`] and [`Draw::fill_textured_polygon`], which
/// differ only in what each vertex's `tex_coord` says. With `textured`, a
/// position is mapped onto the bounding box of the referenced vertices, so the
/// polygon covers the whole texture; without one, `tex_coord` stays `(0, 0)` and
/// the field is ignored.
fn polygon_vertices(
    points: &[(f32, f32)],
    indices: &[i32],
    color: FColor,
    textured: bool,
) -> Vec<Vertex> {
    assert!(
        indices.len() % 3 == 0,
        "Количество индексов должно быть кратно 3"
    );

    let positions: Vec<(f32, f32)> = indices
        .iter()
        .map(|&index| {
            assert!(
                index >= 0 && (index as usize) < points.len(),
                "Индекс вершины {index} выходит за границы массива"
            );
            points[index as usize]
        })
        .collect();

    if positions.is_empty() {
        return Vec::new();
    }

    // Only the vertices the indices reference take part in the box: a point that
    // nothing points at is not part of the polygon and must not stretch the
    // texture over it.
    let (min_x, min_y, w, h) = if textured {
        let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
        let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for &(x, y) in &positions {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        // A box with no width or height means every point sits on one line;
        // dividing by it would poison the coordinates with a NaN.
        (
            min_x,
            min_y,
            (max_x - min_x).max(f32::EPSILON),
            (max_y - min_y).max(f32::EPSILON),
        )
    } else {
        (0.0, 0.0, 1.0, 1.0)
    };

    positions
        .into_iter()
        .map(|(x, y)| Vertex {
            position: FPoint::new(x, y),
            color,
            tex_coord: if textured {
                FPoint::new((x - min_x) / w, (y - min_y) / h)
            } else {
                FPoint::new(0.0, 0.0)
            },
        })
        .collect()
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
        let vertex_color = FColor {
            r: color.r as f32 / 255.0,
            g: color.g as f32 / 255.0,
            b: color.b as f32 / 255.0,
            a: color.a as f32 / 255.0,
        };

        let vertices = polygon_vertices(points, indices, vertex_color, false);
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

    /// Fills the triangles described by `indices` with `image`.
    ///
    /// The polygon's bounding box is stretched over the texture, so the polygon
    /// covers all of it -- the image's aspect ratio is not preserved.
    ///
    /// As in [`fill_texture`](Self::fill_texture), the texture is uploaded to
    /// the GPU on every call and dropped again straight after, so this costs a
    /// texture upload per frame.
    pub fn fill_textured_polygon(
        &mut self,
        points: &[(f32, f32)],
        indices: &[i32],
        image: &crate::image::Image,
    ) -> Result<(), Error> {
        // White modulates nothing, so the image comes through untouched.
        let vertices = polygon_vertices(
            points,
            indices,
            FColor {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            true,
        );
        if vertices.is_empty() {
            return Ok(());
        }

        // `texture_creator` hands over ownership, so the texture borrows the
        // local creator instead of `self.canvas` -- which is what lets
        // `render_geometry` take `&mut self.canvas` below.
        let creator = self.canvas.texture_creator();
        let texture = creator.create_texture_from_surface(&image.surface)?;

        // Unconditional, like in `fill_texture`: unlike a solid colour, an image
        // routinely carries alpha, and without blending it would show up as a
        // hard-edged box.
        self.canvas.set_blend_mode(BlendMode::Blend);

        self.canvas
            .render_geometry(&vertices, Some(&texture), VertexIndices::Sequential)?;

        Ok(())
    }

    /// Fills the box `obj` covers with `color`, turned by its `rotation`.
    ///
    /// The two flip flags are ignored: a solid colour has nothing to mirror.
    pub fn fill_object(&mut self, obj: &Object, color: Color) -> Result<(), Error> {
        if obj.rotation == 0.0 {
            return self.fill_rect(obj.rect(), color);
        }

        let points = obj.corners().map(|p| (p.x, p.y));
        self.fill_polygon(&points, &QUAD_INDICES, color)
    }

    /// Draws `image` as the box `obj` covers, turned by its `rotation` and
    /// mirrored by its flip flags.
    ///
    /// Rotation happens inside SDL, which turns about the centre of the
    /// destination box -- the same point as [`Object::position`].
    ///
    /// As in [`fill_textured_polygon`](Self::fill_textured_polygon), the texture
    /// is uploaded to the GPU on every call and dropped again straight after.
    pub fn fill_textured_object(
        &mut self,
        obj: &Object,
        image: &crate::image::Image,
    ) -> Result<(), Error> {
        // `texture_creator` hands over ownership, so the texture borrows the
        // local creator instead of `self.canvas`, which is what lets the copy
        // below take `&mut self.canvas`.
        let creator = self.canvas.texture_creator();
        let texture = creator.create_texture_from_surface(&image.surface)?;

        // Unconditional, like in `fill_texture`: images routinely carry alpha.
        self.canvas.set_blend_mode(BlendMode::Blend);

        let dst = to_f_rect(obj.rect());
        if obj.rotation == 0.0 && !obj.flip_horizontal && !obj.flip_vertical {
            self.canvas.copy(&texture, None, Some(dst))?;
            return Ok(());
        }

        self.canvas.copy_ex(
            &texture,
            None,
            Some(dst),
            f64::from(obj.rotation),
            None,
            obj.flip_horizontal,
            obj.flip_vertical,
        )?;

        Ok(())
    }

    /// Fills the triangles described by `indices` in `points` with `color`,
    /// turning them about `obj`'s centre.
    ///
    /// `obj` only supplies the centre to turn about: the points stay where they
    /// are otherwise, and `obj.size` is not used.
    pub fn fill_object_polygon(
        &mut self,
        obj: &Object,
        points: &[(f32, f32)],
        indices: &[i32],
        color: Color,
    ) -> Result<(), Error> {
        if obj.rotation == 0.0 {
            return self.fill_polygon(points, indices, color);
        }

        let rotated = rotate_points(points, obj);
        self.fill_polygon(&rotated, indices, color)
    }

    /// Draws the contour through `points`, turning it about `obj`'s centre.
    ///
    /// As in [`fill_object_polygon`](Self::fill_object_polygon), `obj.size` is
    /// not used.
    pub fn draw_object_polygon(
        &mut self,
        obj: &Object,
        points: &[(f32, f32)],
        color: Color,
    ) -> Result<(), Error> {
        if obj.rotation == 0.0 {
            return self.draw_polygon(points, color);
        }

        let rotated = rotate_points(points, obj);
        self.draw_polygon(&rotated, color)
    }

    /// Drawable size of the window, as `(width, height)`.
    pub fn size(&self) -> (u32, u32) {
        self.canvas.window().size()
    }
}
