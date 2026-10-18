//! Axis-aligned rectangles, kept separate from SDL's own type.

/// An axis-aligned rectangle in pixels.
///
/// Coordinates are `f32` because SDL fills rectangles in floating point, and
/// so that this crate's public API does not shift when `sdl3` changes -- the
/// same reasoning as for [`Color`](crate::Color).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    /// A square with its top-left corner at `(x, y)`.
    pub const fn square(x: f32, y: f32, side: f32) -> Self {
        Rect {
            x,
            y,
            w: side,
            h: side,
        }
    }

    /// A square centred on `(cx, cy)`.
    pub const fn square_centered(cx: f32, cy: f32, side: f32) -> Self {
        Rect::square(cx - side / 2.0, cy - side / 2.0, side)
    }

    pub const fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.w, self.h]
    }

    /// Whether the point `(x, y)` is inside, treating the right and bottom
    /// edges as outside.
    ///
    /// Half-open on purpose. A button drawn flush against another must not
    /// share the pixels on their border, or a click on the seam would land on
    /// whichever happened to be drawn first.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

impl From<(f32, f32, f32, f32)> for Rect {
    fn from((x, y, w, h): (f32, f32, f32, f32)) -> Self {
        Rect::new(x, y, w, h)
    }
}

impl From<[f32; 4]> for Rect {
    fn from([x, y, w, h]: [f32; 4]) -> Self {
        Rect::new(x, y, w, h)
    }
}

impl From<(i32, i32, u32, u32)> for Rect {
    fn from((x, y, w, h): (i32, i32, u32, u32)) -> Self {
        Rect::new(x as f32, y as f32, w as f32, h as f32)
    }
}

impl From<Rect> for sdl3::render::FRect {
    fn from(rect: Rect) -> Self {
        sdl3::render::FRect::new(rect.x, rect.y, rect.w, rect.h)
    }
}

#[cfg(test)]
mod tests {
    use super::Rect;

    fn box10() -> Rect {
        Rect::new(10.0, 20.0, 10.0, 10.0)
    }

    /// The inside, including the top-left corner, is in.
    #[test]
    fn a_point_inside_is_inside() {
        let r = box10();

        assert!(r.contains(15.0, 25.0));
        assert!(r.contains(10.0, 20.0), "the top-left corner counts");
        assert!(r.contains(19.999, 29.999));
    }

    /// Anything past an edge is out, including the edge pixels themselves.
    #[test]
    fn a_point_outside_is_outside() {
        let r = box10();

        assert!(!r.contains(9.9, 25.0));
        assert!(!r.contains(20.0, 25.0), "the right edge is out");
        assert!(!r.contains(15.0, 19.9));
        assert!(!r.contains(15.0, 30.0), "the bottom edge is out");
        assert!(!r.contains(-5.0, -5.0));
    }

    /// Half-open is what keeps two flush buttons from sharing a border pixel.
    #[test]
    fn touching_rectangles_do_not_share_their_seam() {
        let left = Rect::new(0.0, 0.0, 10.0, 10.0);
        let right = Rect::new(10.0, 0.0, 10.0, 10.0);
        let seam = 10.0;

        assert!(!left.contains(seam, 5.0));
        assert!(right.contains(seam, 5.0));
    }

    /// A rectangle built empty still behaves, rather than swallowing points.
    #[test]
    fn an_empty_rect_contains_nothing() {
        let empty = Rect::new(5.0, 5.0, 0.0, 10.0);

        assert!(!empty.contains(5.0, 7.0));
    }
}
