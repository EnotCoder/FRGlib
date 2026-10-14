//! Objects: something with a position, a size and a rotation.

use crate::math::Vec2;
use crate::rect::Rect;

/// Something to draw: where it is, how big, how turned, and how it moves.
///
/// An object deliberately holds no texture -- the image is passed separately to
/// the `Draw` methods -- so that one object can be drawn with several textures,
/// which is what a set of directional sprites needs.
///
/// Rotation is about [`position`](Self::position), and flip only means anything
/// for a textured draw.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Object {
    /// Centre of the object, in window pixels.
    pub position: Vec2,
    pub size: Vec2,
    /// Rotation in degrees, clockwise on screen.
    pub rotation: f32,
    /// Movement in pixels per second.
    pub velocity: Vec2,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
}

impl Object {
    /// An object of `size`, centred on `position` and not yet moving or turned.
    pub fn new(position: Vec2, size: Vec2) -> Self {
        Object {
            position,
            size,
            rotation: 0.0,
            velocity: Vec2::ZERO,
            flip_horizontal: false,
            flip_vertical: false,
        }
    }

    /// Sugar for `Object::new(Vec2::new(x, y), Vec2::new(w, h))`.
    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Self {
        Object::new(Vec2::new(x, y), Vec2::new(w, h))
    }

    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    pub fn with_rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }

    pub fn with_flip(mut self, horizontal: bool, vertical: bool) -> Self {
        self.flip_horizontal = horizontal;
        self.flip_vertical = vertical;
        self
    }

    /// Moves the object along its velocity. `dt` is in seconds.
    pub fn update(&mut self, dt: f32) {
        self.position += self.velocity * dt;
    }

    /// The axis-aligned box the object covers when it is not rotated.
    pub fn rect(&self) -> Rect {
        Rect::new(
            self.position.x - self.size.x / 2.0,
            self.position.y - self.size.y / 2.0,
            self.size.x,
            self.size.y,
        )
    }

    /// The four corners, rotated, clockwise from the top left.
    pub fn corners(&self) -> [Vec2; 4] {
        let half = self.size * 0.5;
        let radians = self.rotation.to_radians();

        // Offsets from the centre first, then one rotation for all four: doing
        // it the other way around would repeat the trigonometry four times.
        [
            Vec2::new(-half.x, -half.y),
            Vec2::new(half.x, -half.y),
            Vec2::new(half.x, half.y),
            Vec2::new(-half.x, half.y),
        ]
        .map(|offset| self.position + offset.rotated(radians))
    }

    pub fn turn(&self, point: Vec2) -> Vec2 {
        self.position + (point - self.position).rotated(self.rotation.to_radians())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn assert_close(got: f32, expected: f32) {
        assert!(
            (got - expected).abs() < EPS,
            "expected {expected}, got {got}"
        );
    }

    /// An object at (100, 50), 40 by 20, so half of it is 20 by 10.
    fn sample() -> Object {
        Object::new(Vec2::new(100.0, 50.0), Vec2::new(40.0, 20.0))
    }

    #[test]
    fn new_starts_untouched() {
        let obj = sample();
        assert_eq!(obj.rotation, 0.0);
        assert_eq!(obj.velocity, Vec2::ZERO);
        assert!(!obj.flip_horizontal);
        assert!(!obj.flip_vertical);
    }

    #[test]
    fn from_xywh_matches_new() {
        assert_eq!(
            Object::from_xywh(1.0, 2.0, 3.0, 4.0),
            Object::new(Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0))
        );
    }

    #[test]
    fn builders_set_what_they_say() {
        let obj = sample()
            .with_velocity(Vec2::new(1.0, 2.0))
            .with_rotation(45.0)
            .with_flip(true, false);

        assert_eq!(obj.velocity, Vec2::new(1.0, 2.0));
        assert_eq!(obj.rotation, 45.0);
        assert!(obj.flip_horizontal);
        assert!(!obj.flip_vertical);
    }

    #[test]
    fn rect_sits_around_the_position() {
        let r = sample().rect();
        assert_close(r.x, 80.0);
        assert_close(r.y, 40.0);
        assert_close(r.w, 40.0);
        assert_close(r.h, 20.0);
    }

    #[test]
    fn rect_ignores_rotation() {
        // `rect` is the unrotated box; callers that need the real shape ask for
        // `corners`.
        let mut turned = sample();
        turned.rotation = 90.0;
        assert_eq!(turned.rect(), sample().rect());
    }

    #[test]
    fn corners_run_clockwise_from_the_top_left() {
        let c = sample().corners();

        assert_close(c[0].x, 80.0); // top left
        assert_close(c[0].y, 40.0);
        assert_close(c[1].x, 120.0); // top right
        assert_close(c[1].y, 40.0);
        assert_close(c[2].x, 120.0); // bottom right
        assert_close(c[2].y, 60.0);
        assert_close(c[3].x, 80.0); // bottom left
        assert_close(c[3].y, 60.0);
    }

    #[test]
    fn corners_turn_about_the_centre() {
        let mut obj = sample();
        obj.rotation = 90.0;
        let c = obj.corners();

        // The top left corner sits 20 left and 10 up of the centre; a quarter
        // turn swings it 10 right and 20 up.
        assert_close(c[0].x, 110.0);
        assert_close(c[0].y, 30.0);
    }

    #[test]
    fn turning_keeps_the_centre_where_it_was() {
        let mut obj = sample();
        obj.rotation = 37.0;
        let c = obj.corners();

        let centre_x = (c[0].x + c[1].x + c[2].x + c[3].x) / 4.0;
        let centre_y = (c[0].y + c[1].y + c[2].y + c[3].y) / 4.0;

        assert_close(centre_x, obj.position.x);
        assert_close(centre_y, obj.position.y);
    }

    #[test]
    fn turning_keeps_the_size() {
        let mut obj = sample();
        obj.rotation = 63.0;
        let c = obj.corners();

        // Distance from the centre to a corner is the same before and after.
        let before = Vec2::new(-20.0, -10.0).length();
        let after = (c[0] - obj.position).length();
        assert_close(after, before);
    }

    #[test]
    fn turn_leaves_the_centre_alone() {
        let obj = sample().with_rotation(45.0);
        let got = obj.turn(obj.position);

        assert_close(got.x, obj.position.x);
        assert_close(got.y, obj.position.y);
    }

    #[test]
    fn turn_matches_rotating_about_the_centre() {
        let obj = sample().with_rotation(30.0);
        let point = Vec2::new(140.0, 90.0);

        let relative = point - obj.position;
        let expected = obj.position + relative.rotated(30f32.to_radians());
        let got = obj.turn(point);

        assert_close(got.x, expected.x);
        assert_close(got.y, expected.y);
    }

    #[test]
    fn update_moves_along_the_velocity() {
        let mut obj = sample().with_velocity(Vec2::new(100.0, -50.0));
        obj.update(2.0);

        assert_close(obj.position.x, 300.0);
        assert_close(obj.position.y, -50.0);
    }

    #[test]
    fn update_by_zero_stands_still() {
        let mut obj = sample().with_velocity(Vec2::new(100.0, 50.0));
        obj.update(0.0);

        assert_eq!(obj.position, Vec2::new(100.0, 50.0));
    }
}
