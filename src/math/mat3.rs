use super::Vec2;
use std::ops::Mul;

/// A 3x3 matrix for 2D transformations.
///
/// Uses row-major order internally for indexing, but represents standard 2D
/// affine transformations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3 {
    pub rows: [[f32; 3]; 3],
}

impl Mat3 {
    /// The identity matrix: does nothing when applied.
    pub const IDENTITY: Self = Self {
        rows: [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
    };

    /// A matrix that moves points by `v`.
    pub fn translation(v: Vec2) -> Self {
        let mut m = Self::IDENTITY;
        m.rows[0][2] = v.x;
        m.rows[1][2] = v.y;
        m
    }

    /// A matrix that rotates points by `radians` around the origin (0, 0).
    pub fn rotation(radians: f32) -> Self {
        let (sin, cos) = radians.sin_cos();
        let mut m = Self::IDENTITY;
        m.rows[0][0] = cos;
        m.rows[0][1] = -sin;
        m.rows[1][0] = sin;
        m.rows[1][1] = cos;
        m
    }

    /// A matrix that scales points by `v`.
    pub fn scaling(v: Vec2) -> Self {
        let mut m = Self::IDENTITY;
        m.rows[0][0] = v.x;
        m.rows[1][1] = v.y;
        m
    }

    /// Applies the transformation to a 2D point.
    pub fn transform_point(&self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.rows[0][0] * v.x + self.rows[0][1] * v.y + self.rows[0][2],
            self.rows[1][0] * v.x + self.rows[1][1] * v.y + self.rows[1][2],
        )
    }
}

impl Mul for Mat3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let mut result = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    result[i][j] += self.rows[i][k] * rhs.rows[k][j];
                }
            }
        }
        Self { rows: result }
    }
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::FRAC_PI_2;

    #[test]
    fn identity_does_nothing() {
        let p = Vec2::new(10.0, 20.0);
        assert_eq!(Mat3::IDENTITY.transform_point(p), p);
    }

    #[test]
    fn translation_works() {
        let m = Mat3::translation(Vec2::new(5.0, -3.0));
        let p = Vec2::new(10.0, 20.0);
        assert_eq!(m.transform_point(p), Vec2::new(15.0, 17.0));
    }

    #[test]
    fn rotation_works() {
        let m = Mat3::rotation(FRAC_PI_2);
        let p = Vec2::new(1.0, 0.0);
        let got = m.transform_point(p);
        assert!((got.x - 0.0).abs() < 1e-6);
        assert!((got.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn scaling_works() {
        let m = Mat3::scaling(Vec2::new(2.0, 0.5));
        let p = Vec2::new(10.0, 20.0);
        assert_eq!(m.transform_point(p), Vec2::new(20.0, 10.0));
    }

    #[test]
    fn matrix_multiplication_composes() {
        let t = Mat3::translation(Vec2::new(10.0, 0.0));
        let r = Mat3::rotation(FRAC_PI_2);
        
        // Translate then rotate
        let combined = r * t;
        let p = Vec2::new(0.0, 0.0);
        let got = combined.transform_point(p);
        
        // (0,0) -> translate -> (10,0) -> rotate -> (0,10)
        assert!((got.x - 0.0).abs() < 1e-6);
        assert!((got.y - 10.0).abs() < 1e-6);
    }
}
