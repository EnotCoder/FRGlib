use frglib::Vec2;

#[derive(Clone, Copy)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn project(
    point: Point3,
    center: Vec2,
    camera_distance: f32,
    focal_length: f32,
) -> Vec2 {
    let depth = camera_distance + point.z;
    let scale = focal_length / depth;

    Vec2::new(
        center.x + point.x * scale,
        center.y - point.y * scale,
    )
}

// Подготовленные примитивы

pub fn get_rect_vert(size: f32) -> [Point3; 4] {
    [
        Point3 {
            x: -size,
            y: -size,
            z: 0.0,
        },
        Point3 {
            x: size,
            y: -size,
            z: 0.0,
        },
        Point3 {
            x: size,
            y: size,
            z: 0.0,
        },
        Point3 {
            x: -size,
            y: size,
            z: 0.0,
        },
    ]
}
