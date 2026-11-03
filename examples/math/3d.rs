use frglib::{
    Color, Event, FrameClock, Keycode, Object, Window, Vec2,
};

#[derive(Clone, Copy)]
struct Point3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy)]
struct Mat4 {
    rows: [[f32; 4]; 4],
}

impl Mat4 {
    const IDENTITY: Self = Self {
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

fn rotation_x(angle: f32) -> Mat4 {
    let (s, c) = angle.sin_cos();
    let mut m = Mat4::IDENTITY;

    m.rows[1][1] = c;
    m.rows[1][2] = -s;
    m.rows[2][1] = s;
    m.rows[2][2] = c;

    m
}

fn rotation_y(angle: f32) -> Mat4 {
    let (s, c) = angle.sin_cos();
    let mut m = Mat4::IDENTITY;

    m.rows[0][0] = c;
    m.rows[0][2] = s;
    m.rows[2][0] = -s;
    m.rows[2][2] = c;

    m
}

// Перемещение по всем трём осям.
fn translation(x: f32, y: f32, z: f32) -> Mat4 {
    let mut m = Mat4::IDENTITY;

    m.rows[0][3] = x;
    m.rows[1][3] = y;
    m.rows[2][3] = z;

    m
}

// Перемещение только по Y.
fn position_y(num: f32) -> Mat4 {
    translation(0.0, num, 0.0)
}

// Применяем матрицу к точке (x, y, z, 1).
// Функция рассчитана на вращение и перенос, не на перспективную матрицу.
fn transform(matrix: Mat4, point: Point3) -> Point3 {
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
        let angle_y = elapsed;

        // Плавное перемещение вверх-вниз.
        let offset_y = elapsed.sin() * 100.0;

        // Справа налево:
        // 1. Поворот вокруг X.
        // 2. Поворот вокруг Y.
        // 3. Перемещение по мировой оси Y.
        let model = position_y(offset_y)
            * rotation_y(angle_y)
            * rotation_x(angle_x);

        let mut screen_points = Vec::with_capacity(vertices.len());

        for &vertex in &vertices {
            let transformed = transform(model, vertex);

            let p = project(
                transformed,
                center,
                camera_distance,
                focal_length,
            );

            screen_points.push((p.x, p.y));
        }

        let dummy_obj = Object::new(Vec2::ZERO, Vec2::ZERO);

        window.frame_with(|d| {
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
