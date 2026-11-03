use crate::Point3;
use crate::Mat4;

// Поворот
fn rotation_x(angle: f32) -> Mat4 {
    let (s, c) = angle.sin_cos();
    let mut matrix = Mat4::IDENTITY;

    matrix.rows[1][1] = c;
    matrix.rows[1][2] = -s;
    matrix.rows[2][1] = s;
    matrix.rows[2][2] = c;

    return matrix;
}

fn rotation_y(angle: f32) -> Mat4 {
    let (s, c) = angle.sin_cos();
    let mut matrix = Mat4::IDENTITY;

    matrix.rows[0][0] = c;
    matrix.rows[0][2] = s;
    matrix.rows[2][0] = -s;
    matrix.rows[2][2] = c;

    return matrix;
}

fn rotation_z(angle: f32) -> Mat4 {
    let (s, c) = angle.sin_cos();
    let mut matrix = Mat4::IDENTITY;

    matrix.rows[0][0] = c;
    matrix.rows[0][1] = s;
    matrix.rows[1][0] = -s;
    matrix.rows[1][1] = c;

    return matrix;
}

pub fn rotation(x: f32, y: f32, z: f32) -> Mat4 {
    rotation_x(x) * rotation_y(y) * rotation_z(z)
}

// Перемещение
pub fn translation(x: f32, y: f32, z: f32) -> Mat4 {
    let mut matrix = Mat4::IDENTITY;

    matrix.rows[0][3] = x;
    matrix.rows[1][3] = y;
    matrix.rows[2][3] = z;

    return matrix;
}

// Применяем матрицу к точке (x, y, z, 1).
pub fn transform(matrix: Mat4, point: Point3) -> Point3 {
    Point3 {
        x: matrix.rows[0][0] * point.x
            + matrix.rows[0][1] * point.y
            + matrix.rows[0][2] * point.z
            + matrix.rows[0][3],

        y: matrix.rows[1][0] * point.x
            + matrix.rows[1][1] * point.y
            + matrix.rows[1][2] * point.z
            + matrix.rows[1][3],

        z: matrix.rows[2][0] * point.x
            + matrix.rows[2][1] * point.y
            + matrix.rows[2][2] * point.z
            + matrix.rows[2][3],
    }
}
