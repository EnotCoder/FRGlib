#[derive(Clone, Copy)]
pub struct Mat4 {
    pub rows: [[f32; 4]; 4],
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        rows: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };
}

// Позволяет matrix_a * matrix_b.
impl std::ops::Mul for Mat4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let mut result = Self {
            rows: [[0.0; 4]; 4],
        };

        for row in 0..4 {
            for col in 0..4 {
                for k in 0..4 {
                    result.rows[row][col] +=
                        self.rows[row][k] * rhs.rows[k][col];
                }
            }
        }

        result
    }
}
