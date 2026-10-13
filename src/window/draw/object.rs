//! The `*_object` methods -- the whole drawing API, in other words.
//!
//! Each one hands over to a private primitive in the parent module, so the
//! difference between them is only what an [`Object`] contributes: a centre to
//! turn about, a size, and two flip flags.

use super::geometry::{polygon_vertices, rotate_points, to_f_rect, QUAD_INDICES, WHITE};
use super::Draw;

use crate::color::Color;
use crate::math::Vec2;
use crate::object::Object;
use crate::window::Error;

use sdl3::render::{BlendMode, FPoint, VertexIndices};

impl<'a> Draw<'a> {
    /// Fills the triangles described by `indices` in `points` with `image`,
    /// turning them about `obj`'s centre.
    ///
    /// The texture is stretched over the polygon's bounding box as it was
    /// *before* the turn, so the image covers the same shape it would cover
    /// unrotated and simply goes round with it.
    ///
    /// The texture is uploaded to the GPU on every call and dropped again
    /// straight after, so this costs a texture upload per frame.
    pub fn fill_textured_object_polygon(
        &mut self,
        obj: &Object,
        points: &[(f32, f32)],
        indices: &[i32],
        image: &crate::image::Image,
    ) -> Result<(), Error> {
        // Texture coordinates come from the polygon as given, while the
        // positions get turned -- hence the turn inside the builder rather than
        // on `points` beforehand, which would stretch the texture along with
        // the shape.
        let mut vertices = polygon_vertices(points, indices, WHITE, true, Some(obj));
        if vertices.is_empty() {
            return Ok(());
        }

        // Placed through the camera after that, so a vertex is turned once and
        // projected once. Texture coordinates are left alone throughout: they
        // address the image, not the world.
        if let Some(camera) = self.camera {
            let size = self.size_px();
            for vertex in &mut vertices {
                let p = camera.to_screen(Vec2::new(vertex.position.x, vertex.position.y), size);
                vertex.position = FPoint::new(p.x, p.y);
            }
        }

        // `texture_creator` hands over ownership, so the texture borrows the
        // local creator instead of `self.canvas` -- which is what lets
        // `render_geometry` take `&mut self.canvas` below.
        let creator = self.canvas.texture_creator();
        let texture = creator.create_texture_from_surface(&image.surface)?;

        // Unconditional: unlike a solid colour, an image routinely carries
        // alpha, and without blending it would come out as a hard-edged box.
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
    /// The box is projected through the camera first, so the angle is left
    /// alone: the zoom is the same on both axes, so it scales the box without
    /// turning it.
    ///
    /// The texture is uploaded to the GPU on every call and dropped again
    /// straight after, so this costs a texture upload per frame.
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

        // Unconditional: images routinely carry alpha, and without blending they
        // would come out as hard-edged boxes.
        self.canvas.set_blend_mode(BlendMode::Blend);

        let dst = to_f_rect(self.place_rect(obj.rect()));
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

    /// Draws the contour of the box `obj` covers, turned by its `rotation`.
    ///
    /// The counterpart of [`fill_object`](Self::fill_object): same box, same
    /// centre, outline instead of fill.
    pub fn draw_object(&mut self, obj: &Object, color: Color) -> Result<(), Error> {
        let points = obj.corners().map(|p| (p.x, p.y));
        self.draw_polygon(&points, color)
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
}
