use frglib::{
    Color, Event, FrameClock, Keycode, Mat3, Object, Vec2, Window,
};

// Собственный 3D-вектор для этого примера.
#[derive(Clone, Copy)]
struct Point3 {
    x: f32,
    y: f32,
    z: f32,
}

fn rotation_x(angle: f32) -> Mat3 {
    let (s, c) = angle.sin_cos();
    let mut m = Mat3::IDENTITY;
    m.rows[1][1] = c;
    m.rows[1][2] = -s;
    m.rows[2][1] = s;
    m.rows[2][2] = c;
    m
}

fn rotation_y(angle: f32) -> Mat3 {
    let (s, c) = angle.sin_cos();
    let mut m = Mat3::IDENTITY;
    m.rows[0][0] = c;
    m.rows[0][2] = s;
    m.rows[2][0] = -s;
    m.rows[2][2] = c;
    m
}

// Применение матрицы к 3D-вектору.
fn transform(matrix: Mat3, point: Point3) -> Point3 {
    Point3 {
        x: matrix.rows[0][0] * point.x
            + matrix.rows[0][1] * point.y
            + matrix.rows[0][2] * point.z,

        y: matrix.rows[1][0] * point.x
            + matrix.rows[1][1] * point.y
            + matrix.rows[1][2] * point.z,

        z: matrix.rows[2][0] * point.x
            + matrix.rows[2][1] * point.y
            + matrix.rows[2][2] * point.z,
    }
}

// Перспективная проекция: 3D-точка → координаты окна.
fn project(
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

fn main() -> Result<(), frglib::Error> {
    let mut window = Window::builder("FRGlib: rotating 3D polygon")
        .size(900, 700)
        .background(Color::rgb(18, 20, 30))
        .build()?;

    let mut clock = FrameClock::new(60.0);
    let mut elapsed = 0.0f32;

    // Квадрат лежит в плоскости XY.
    // Пока у всех вершин z = 0.
    let vertices = [
        Point3 {
            x: -130.0,
            y: -130.0,
            z: 0.0,
        },
        Point3 {
            x: 130.0,
            y: -130.0,
            z: 0.0,
        },
        Point3 {
            x: 130.0,
            y: 130.0,
            z: 0.0,
        },
        Point3 {
            x: -130.0,
            y: 130.0,
            z: 0.0,
        },
    ];

    let camera_distance = 600.0;
    let focal_length = 600.0;
    let indices = [0, 1, 2, 0, 2, 3];

    loop {
        elapsed += clock.dt();

        for event in window.poll()? {
            match event {
                Event::Quit | Event::CloseRequested => {
                    return Ok(());
                }

                Event::KeyDown {
                    key: Keycode::Escape,
                    ..
                } => {
                    return Ok(());
                }

                _ => {}
            }
        }

        let (width, height) = window.size();

        let center = Vec2::new(
            width as f32 / 2.0,
            height as f32 / 2.0,
        );

        let angle_x = elapsed * 0.7;
        let angle_y = elapsed * 1.0;

        // Используем оператор * для Mat3.
        let rotation = rotation_y(angle_y) * rotation_x(angle_x);

        let mut screen_points = Vec::with_capacity(4);

        for i in 0..vertices.len() {
            let rotated = transform(rotation, vertices[i]);

            let p = project(
                rotated,
                center,
                camera_distance,
                focal_length,
            );
            screen_points.push((p.x, p.y));
        }

        let dummy_obj = Object::new(Vec2::ZERO, Vec2::ZERO);

        window.frame_with(|d| {
            // Используем публичный API через Object.
            d.fill_object_polygon(
                &dummy_obj,
                &screen_points,
                &indices,
                Color::rgb(100, 170, 255),
            )?;

            d.draw_object_polygon(
                &dummy_obj,
                &screen_points,
                Color::rgb(255, 255, 255),
            )?;

            Ok(())
        })?;

        clock.tick();
    }
}