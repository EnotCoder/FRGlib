//! The camera: which part of the world the window shows.

use crate::math::Rect;
use crate::math::Vec2;

/// What the window looks at.
///
/// Drawing takes place in world coordinates and the camera decides where that
/// lands on screen. The camera is unconstrained: it may go anywhere, and the
/// window will happily show whatever is there, or nothing.
///
/// Note that [`Object::size`](crate::Object::size) and positions are in world
/// units, so a 64-wide object covers `64 * zoom` pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Centre of the view, in world coordinates.
    pub position: Vec2,

    /// Magnification: `1.0` draws one world unit to one pixel, `2.0` twice as
    /// big, and so on.
    pub zoom: f32,
}

impl Camera {
    /// An unzoomed camera looking at `position`.
    pub fn new(position: Vec2) -> Self {
        Camera {
            position,
            zoom: 1.0,
        }
    }

    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// The zoom to divide and multiply by, kept away from zero.
    ///
    /// A zero or negative zoom has no sensible reading -- `to_world` would
    /// divide by it -- so anything below `EPSILON` counts as no zoom at all.
    fn scale(&self) -> f32 {
        if self.zoom > f32::EPSILON {
            self.zoom
        } else {
            1.0
        }
    }

    /// Where `world` lands on screen. Used by the drawing methods.
    pub(crate) fn to_screen(self, world: Vec2, size: (u32, u32)) -> Vec2 {
        let scale = self.scale();
        let centre = Vec2::new(size.0 as f32 / 2.0, size.1 as f32 / 2.0);

        (world - self.position) * scale + centre
    }

    /// The inverse of `to_screen`.
    ///
    /// This is what turns a point from a mouse event into a place in the world.
    /// `size` has to be the window size the event was measured against.
    pub fn to_world(self, screen: Vec2, size: (u32, u32)) -> Vec2 {
        let scale = self.scale();
        let centre = Vec2::new(size.0 as f32 / 2.0, size.1 as f32 / 2.0);

        (screen - centre) / scale + self.position
    }

    /// Projects a world-space box onto the screen.
    ///
    /// Only the top left corner goes through [`to_screen`](Self::to_screen):
    /// the size is scaled on its own, which is exact because the zoom is the
    /// same on both axes. This assumes the box is not rotated -- a turned box
    /// is a quad, and its four corners have to be projected one by one.
    pub(crate) fn project_rect(self, rect: Rect, size: (u32, u32)) -> Rect {
        let corner = self.to_screen(Vec2::new(rect.x, rect.y), size);
        let scale = self.scale();

        Rect::new(corner.x, corner.y, rect.w * scale, rect.h * scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: (u32, u32) = (800, 600);
    const CENTRE: Vec2 = Vec2::new(400.0, 300.0);

    fn assert_close(got: f32, expected: f32) {
        assert!(
            (got - expected).abs() < 1e-4,
            "expected {expected}, got {got}"
        );
    }

    #[test]
    fn new_is_unzoomed() {
        let camera = Camera::new(Vec2::new(10.0, 20.0));
        assert_eq!(camera.position, Vec2::new(10.0, 20.0));
        assert_eq!(camera.zoom, 1.0);
    }

    #[test]
    fn the_looked_at_point_sits_in_the_middle_of_the_window() {
        let camera = Camera::new(Vec2::new(10.0, 20.0));
        let got = camera.to_screen(camera.position, SIZE);
        assert_close(got.x, CENTRE.x);
        assert_close(got.y, CENTRE.y);
    }

    #[test]
    fn an_unzoomed_camera_only_shifts_by_the_centre() {
        let camera = Camera::new(Vec2::new(0.0, 0.0));
        let got = camera.to_screen(Vec2::new(10.0, 20.0), SIZE);
        assert_close(got.x, CENTRE.x + 10.0);
        assert_close(got.y, CENTRE.y + 20.0);
    }

    #[test]
    fn zoom_doubles_the_distance_from_the_centre() {
        let camera = Camera::new(Vec2::ZERO).with_zoom(2.0);
        let got = camera.to_screen(Vec2::new(100.0, 50.0), SIZE);

        assert_close(got.x, CENTRE.x + 200.0);
        assert_close(got.y, CENTRE.y + 100.0);
    }

    #[test]
    fn zoom_does_not_move_the_centre_itself() {
        let camera = Camera::new(Vec2::new(7.0, 9.0)).with_zoom(3.5);
        let got = camera.to_screen(camera.position, SIZE);
        assert_close(got.x, CENTRE.x);
        assert_close(got.y, CENTRE.y);
    }

    #[test]
    fn screen_and_world_are_inverses() {
        let camera = Camera::new(Vec2::new(-30.0, 12.0)).with_zoom(1.75);

        let world = Vec2::new(123.0, -456.0);
        let screen = camera.to_screen(world, SIZE);
        let back = camera.to_world(screen, SIZE);

        assert_close(back.x, world.x);
        assert_close(back.y, world.y);
    }

    #[test]
    fn a_zero_zoom_behaves_like_no_zoom_rather_than_exploding() {
        // to_world divides by the zoom, so anything below EPSILON has to fall
        // back to 1.0 or it would hand back infinities.
        let camera = Camera::new(Vec2::ZERO).with_zoom(0.0);
        let got = camera.to_world(CENTRE + Vec2::new(10.0, 20.0), SIZE);

        assert!(got.x.is_finite());
        assert!(got.y.is_finite());
        assert_close(got.x, 10.0);
        assert_close(got.y, 20.0);
    }

    #[test]
    fn a_negative_zoom_is_treated_as_no_zoom() {
        let camera = Camera::new(Vec2::ZERO).with_zoom(-4.0);
        let got = camera.to_screen(Vec2::new(30.0, 40.0), SIZE);
        assert_close(got.x, CENTRE.x + 30.0);
        assert_close(got.y, CENTRE.y + 40.0);
    }

    #[test]
    fn moving_the_camera_moves_what_is_on_screen() {
        let world = Vec2::new(100.0, 100.0);

        let here = Camera::new(Vec2::ZERO).to_screen(world, SIZE);
        let there = Camera::new(Vec2::new(50.0, 0.0)).to_screen(world, SIZE);

        // Looking further along x shows the same point further left.
        assert_close(there.x, here.x - 50.0);
        assert_close(there.y, here.y);
    }
}
