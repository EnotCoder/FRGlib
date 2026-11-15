//! Мелкие геометрические помощники, которых нет в `frglib::Draw`.
//!
//! Всё сводится к `fill_object_polygon` / `draw_object_polygon`: один вызов
//! строит вершины, второй рисует контур. Батчинг квадов здесь особенно важен:
//! пояса и звёзды -- это тысячи точек, и по вызову на каждую кадр не тянет.

use frglib::{Color, Draw, Object, Vec2};
use std::f32::consts::TAU;

/// Сегментов в полном круге. Хватает, чтобы край не выглядел гранёным.
pub const CIRCLE_SEGMENTS: usize = 48;

/// Один вызов рисует все квады из списка.
///
/// `quads` -- центр и сторона каждого квадрата в мировых координатах.
/// Цвет у всех общий, поэтому вызывающий сам группирует точки по оттенку.
pub fn fill_quads(d: &mut Draw, quads: &[(Vec2, f32)], color: Color) -> Result<(), frglib::Error> {
    if quads.is_empty() {
        return Ok(());
    }

    let mut points = Vec::with_capacity(quads.len() * 4);
    let mut indices = Vec::with_capacity(quads.len() * 6);

    for (i, (position, size)) in quads.iter().enumerate() {
        let half = *size * 0.5;
        let base = (i * 4) as i32;

        points.extend_from_slice(&[
            (position.x - half, position.y - half),
            (position.x + half, position.y - half),
            (position.x + half, position.y + half),
            (position.x - half, position.y + half),
        ]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    d.fill_object_polygon(
        &Object::new(Vec2::ZERO, Vec2::ZERO),
        &points,
        &indices,
        color,
    )
}

/// Залитый круг.
pub fn fill_disc(
    d: &mut Draw,
    center: Vec2,
    radius: f32,
    color: Color,
) -> Result<(), frglib::Error> {
    let mut points = Vec::with_capacity(CIRCLE_SEGMENTS + 1);
    let mut indices = Vec::with_capacity(CIRCLE_SEGMENTS * 3);

    points.push((center.x, center.y));

    for i in 0..CIRCLE_SEGMENTS {
        let angle = TAU * i as f32 / CIRCLE_SEGMENTS as f32;

        points.push((
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        ));
    }

    for i in 0..CIRCLE_SEGMENTS {
        let next = (i + 1) % CIRCLE_SEGMENTS;
        indices.extend_from_slice(&[0, 1 + i as i32, 1 + next as i32]);
    }

    d.fill_object_polygon(&Object::new(center, Vec2::ZERO), &points, &indices, color)
}

/// Половина круга -- ночная сторона планеты.
///
/// `sun_angle` -- направление на Солнце. Затемняется противоположная
/// половина, поэтому терминатор всегда смотрит от светила.
pub fn fill_night_side(
    d: &mut Draw,
    center: Vec2,
    radius: f32,
    sun_angle: f32,
    color: Color,
) -> Result<(), frglib::Error> {
    const SEGMENTS: usize = 24;

    let start = sun_angle + TAU / 4.0;
    let mut points = Vec::with_capacity(SEGMENTS + 2);
    let mut indices = Vec::with_capacity(SEGMENTS * 3);

    points.push((center.x, center.y));

    for i in 0..=SEGMENTS {
        let angle = start + std::f32::consts::PI * i as f32 / SEGMENTS as f32;

        points.push((
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius,
        ));
    }

    for i in 1..=SEGMENTS {
        indices.extend_from_slice(&[0, i as i32, i as i32 + 1]);
    }

    d.fill_object_polygon(&Object::new(center, Vec2::ZERO), &points, &indices, color)
}

/// Контур эллипса от `start` до `end` (радианы), нарисованный толстой
/// линией из квадов.
///
/// `flatten` сжимает вертикаль: `1.0` -- окружность, [`crate::consts::ORBIT_TILT`]
/// -- орбита, которую видно чуть сбоку. Один вызов на всю дугу: тысячи
/// коротких `draw_line` стоят заметно дороже.
#[allow(clippy::too_many_arguments)]
pub fn stroke_ellipse_arc(
    d: &mut Draw,
    center: Vec2,
    radius: f32,
    flatten: f32,
    start: f32,
    end: f32,
    segments: usize,
    width: f32,
    color: Color,
) -> Result<(), frglib::Error> {
    let segments = segments.max(2);
    let half_width = (width * 0.5).max(0.01);

    let mut points = Vec::with_capacity(segments * 4);
    let mut indices = Vec::with_capacity(segments * 6);

    let mut previous = Vec2::new(
        center.x + start.cos() * radius,
        center.y + start.sin() * radius * flatten,
    );

    for i in 1..=segments {
        let angle = start + (end - start) * i as f32 / segments as f32;
        let current = Vec2::new(
            center.x + angle.cos() * radius,
            center.y + angle.sin() * radius * flatten,
        );

        let delta = current - previous;
        let length = delta.length();

        if length > f32::EPSILON {
            let normal = Vec2::new(-delta.y, delta.x) / length * half_width;
            let base = points.len() as i32;

            points.extend_from_slice(&[
                (previous.x + normal.x, previous.y + normal.y),
                (previous.x - normal.x, previous.y - normal.y),
                (current.x - normal.x, current.y - normal.y),
                (current.x + normal.x, current.y + normal.y),
            ]);
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        previous = current;
    }

    d.fill_object_polygon(&Object::new(center, Vec2::ZERO), &points, &indices, color)
}

/// Половина плоского кольца -- кольца Сатурна.
///
/// `front` выбирает нижнюю (ближнюю) или верхнюю (дальнюю) половину:
/// задняя рисуется до планеты, передняя -- после, и кольцо оказывается
/// и за планетой, и перед ней.
#[allow(clippy::too_many_arguments)]
pub fn fill_ellipse_ring_half(
    d: &mut Draw,
    center: Vec2,
    inner_radius: f32,
    outer_radius: f32,
    flatten: f32,
    front: bool,
    color: Color,
) -> Result<(), frglib::Error> {
    const SEGMENTS: usize = 64;

    let (start, end) = if front {
        (0.0, std::f32::consts::PI)
    } else {
        (std::f32::consts::PI, TAU)
    };

    let mut points = Vec::with_capacity((SEGMENTS + 1) * 2);
    let mut indices = Vec::with_capacity(SEGMENTS * 6);

    for i in 0..=SEGMENTS {
        let angle = start + (end - start) * i as f32 / SEGMENTS as f32;
        let (sin, cos) = angle.sin_cos();

        points.push((
            center.x + cos * outer_radius,
            center.y + sin * outer_radius * flatten,
        ));
        points.push((
            center.x + cos * inner_radius,
            center.y + sin * inner_radius * flatten,
        ));
    }

    for i in 0..SEGMENTS {
        let base = (i * 2) as i32;
        indices.extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);
    }

    d.fill_object_polygon(&Object::new(center, Vec2::ZERO), &points, &indices, color)
}
