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
