//! Two-dimensional points, vectors and boxes, and moving between values.
//!
//! [`Vec2`] lives here directly; [`Rect`] is a box in the same coordinate
//! space, so its module sits alongside rather than at the crate root.
//! [`lerp`] and [`Ease`] are here too: they are generic over everything that
//! interpolates, [`Color`](crate::Color) among them, rather than owned by any
//! one type.

pub mod ease;
pub mod rect;

pub use ease::Ease;
pub use rect::Rect;

/// A point or vector in pixels.
///
/// Floats rather than integers, for the same reason [`Rect`] is:
/// SDL works in floating point, and keeping `f32` here means the public API does
/// not shift when `sdl3` changes.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }

    /// Length squared, without the sqrt. Use it when comparing distances.
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Length with the sqrt. Use it when you need the real number.
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Two vectors multiplied into one number: how much they point the same way.
    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Length made 1, direction unchanged. A zero vector stays zero.
    pub fn normalized(self) -> Vec2 {
        let len = self.length();
        if len == 0.0 {
            Vec2::ZERO
        } else {
            self / len
        }
    }

    /// Turns the vector by an angle in radians, not degrees
    pub fn rotated(self, radians: f32) -> Vec2 {
        let (sin, cos) = radians.sin_cos();
        Vec2::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }

    pub const fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

// operator plus.
impl std::ops::Add for Vec2 {
    /// result type is Vec2.
    type Output = Vec2;

    /// `self` is the number left of the plus, `other` the one right of it.
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

// operator minus.
impl std::ops::Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

/// Multiplies both coordinates by a number.
impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, scalar: f32) -> Vec2 {
        Vec2::new(self.x * scalar, self.y * scalar)
    }
}

/// Divides both coordinates by the number.
impl std::ops::Div<f32> for Vec2 {
    type Output = Vec2;

    fn div(self, scalar: f32) -> Vec2 {
        Vec2::new(self.x / scalar, self.y / scalar)
    }
}

// Changes the signs of both coordinates.
impl std::ops::Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

/// Adds `other` to `self`, writing the sum back into it.
impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        *self = *self + other;
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from((x, y): (f32, f32)) -> Self {
        Vec2::new(x, y)
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from([x, y]: [f32; 2]) -> Self {
        Vec2::new(x, y)
    }
}

/// A value that knows how to walk from itself towards another one.
///
/// [`lerp`] is the generic way to call it; the trait method is the same thing
/// spelled as a method -- `from.lerp(to, t, ease)` -- for when a chain reads
/// better. The method form needs this trait in scope:
///
/// ```
/// use frglib::{Ease, Lerp, Vec2};
///
/// let halfway = Vec2::ZERO.lerp(Vec2::new(10.0, 20.0), 0.5, Ease::Linear);
/// assert_eq!(halfway, Vec2::new(5.0, 10.0));
/// ```
pub trait Lerp: Sized {
    /// The point `t` of the way from `self` to `other`, shaped by `ease`.
    ///
    /// `t` is not clamped: `0.0` gives `self`, `1.0` gives `other`, and a
    /// weight outside that range extrapolates past the endpoints -- which is
    /// what Godot's `lerp` does too. [`Color`](crate::Color) is the exception:
    /// its channels stop at `0` and `255`, because `u8` cannot hold more.
    fn lerp(self, other: Self, t: f32, ease: Ease) -> Self;
}

/// [`Lerp::lerp`] as a free function: `lerp(from, to, t, ease)`.
///
/// The method form and this one are the same call, so pick whichever reads
/// better where you are:
///
/// ```
/// use frglib::{lerp, Ease, Lerp, Vec2};
///
/// let from = Vec2::new(10.0, 20.0);
/// let to = Vec2::new(30.0, 60.0);
///
/// assert_eq!(lerp(from, to, 0.5, Ease::Linear), Vec2::new(20.0, 40.0));
/// assert_eq!(from.lerp(to, 0.5, Ease::Linear), Vec2::new(20.0, 40.0));
/// ```
pub fn lerp<T: Lerp>(from: T, to: T, t: f32, ease: Ease) -> T {
    from.lerp(to, t, ease)
}

/// Sine of `radians`, as in Godot.
pub fn sin(radians: f32) -> f32 {
    radians.sin()
}

/// Cosine of `radians`, as in Godot.
pub fn cos(radians: f32) -> f32 {
    radians.cos()
}

impl Lerp for f32 {
    fn lerp(self, other: f32, t: f32, ease: Ease) -> f32 {
        let t = ease.apply(t);
        self + (other - self) * t
    }
}

impl Lerp for Vec2 {
    fn lerp(self, other: Vec2, t: f32, ease: Ease) -> Vec2 {
        let t = ease.apply(t);
        self + (other - self) * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How far two numbers may differ and still count as equal in a test.
    const EPS: f32 = 1e-4;

    fn assert_close(got: f32, expected: f32) {
        assert!(
            (got - expected).abs() < EPS,
            "expected {expected}, got {got}"
        );
    }

    #[test]
    fn conversions_round_trip() {
        assert_eq!(Vec2::new(3.0, 4.0).to_array(), [3.0, 4.0]);
        assert_eq!(Vec2::from((3.0, 4.0)), Vec2::new(3.0, 4.0));
        assert_eq!(Vec2::from([3.0, 4.0]), Vec2::new(3.0, 4.0));
        assert_eq!(Vec2::default(), Vec2::ZERO);
    }

    #[test]
    fn arithmetic() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, 4.0);

        assert_eq!(a + b, Vec2::new(4.0, 6.0));
        assert_eq!(b - a, Vec2::new(2.0, 2.0));
        assert_eq!(a * 2.0, Vec2::new(2.0, 4.0));
        assert_eq!(a / 2.0, Vec2::new(0.5, 1.0));
        assert_eq!(-a, Vec2::new(-1.0, -2.0));

        let mut acc = a;
        acc += b;
        assert_eq!(acc, Vec2::new(4.0, 6.0));
    }

    #[test]
    fn lengths_and_dot() {
        let v = Vec2::new(3.0, 4.0);
        assert_close(v.length(), 5.0);
        assert_close(v.length_squared(), 25.0);

        assert_close(Vec2::new(2.0, 0.0).dot(Vec2::new(3.0, 0.0)), 6.0);
        assert_close(Vec2::new(1.0, 0.0).dot(Vec2::new(0.0, 1.0)), 0.0);
    }

    #[test]
    fn normalizing_gives_unit_length() {
        assert_close(Vec2::new(3.0, 4.0).normalized().length(), 1.0);
    }

    #[test]
    fn normalizing_a_zero_vector_gives_zero() {
        // Otherwise this divides by its own length.
        assert_eq!(Vec2::ZERO.normalized(), Vec2::ZERO);
    }

    #[test]
    fn turning_by_ninety_degrees() {
        let got = Vec2::new(1.0, 0.0).rotated(std::f32::consts::FRAC_PI_2);
        assert_close(got.x, 0.0);
        assert_close(got.y, 1.0);
    }

    #[test]
    fn turning_by_nothing_changes_nothing() {
        let got = Vec2::new(3.0, -4.0).rotated(0.0);
        assert_close(got.x, 3.0);
        assert_close(got.y, -4.0);
    }

    #[test]
    fn turning_preserves_length() {
        // A rotation is rigid: it must not stretch the vector it turns.
        let v = Vec2::new(3.0, 4.0);
        assert_close(v.rotated(0.7).length(), v.length());
    }

    #[test]
    fn turning_half_a_turn_reverses_the_vector() {
        let got = Vec2::new(2.0, 5.0).rotated(std::f32::consts::PI);
        assert_close(got.x, -2.0);
        assert_close(got.y, -5.0);
    }

    #[test]
    fn turning_back_cancels_out() {
        let v = Vec2::new(2.0, 5.0);
        let got = v
            .rotated(std::f32::consts::FRAC_PI_2)
            .rotated(-std::f32::consts::FRAC_PI_2);
        assert_close(got.x, v.x);
        assert_close(got.y, v.y);
    }

    #[test]
    fn lerping_a_scalar_walks_at_a_constant_rate() {
        assert_eq!(lerp(0.0f32, 10.0, 0.0, Ease::Linear), 0.0);
        assert_eq!(lerp(0.0f32, 10.0, 0.25, Ease::Linear), 2.5);
        assert_eq!(lerp(0.0f32, 10.0, 1.0, Ease::Linear), 10.0);
    }

    #[test]
    fn lerping_a_vector_moves_both_coordinates() {
        let from = Vec2::new(10.0, 20.0);
        let to = Vec2::new(30.0, 60.0);

        assert_eq!(lerp(from, to, 0.0, Ease::Linear), from);
        assert_eq!(lerp(from, to, 1.0, Ease::Linear), to);
        assert_eq!(lerp(from, to, 0.5, Ease::Linear), Vec2::new(20.0, 40.0));
    }

    /// The weight stays a raw number: past `1.0` means past the target, the
    /// way Godot's `lerp` carries on instead of stopping at the endpoint.
    #[test]
    fn the_weight_is_not_clamped() {
        assert_eq!(lerp(0.0f32, 10.0, 2.0, Ease::Linear), 20.0);
        assert_eq!(lerp(0.0f32, 10.0, -1.0, Ease::Linear), -10.0);
    }

    /// The free function and the method are one call spelled two ways, so
    /// neither is allowed to drift from the other.
    #[test]
    fn the_function_and_the_method_agree() {
        let from = Vec2::new(1.0, 2.0);
        let to = Vec2::new(5.0, -3.0);

        assert_eq!(
            lerp(from, to, 0.3, Ease::Linear),
            from.lerp(to, 0.3, Ease::Linear)
        );
        assert_eq!(
            lerp(4.0f32, 9.0, 0.3, Ease::Linear),
            (4.0f32).lerp(9.0, 0.3, Ease::Linear)
        );
    }
}
