//! Vertex buffers and the arithmetic behind them.
//!
//! Nothing here touches a canvas: everything takes plain numbers and gives back
//! either an SDL type or a `Vec`, so the drawing methods in the parent module
//! stay a thin layer over these.

use crate::graphics::Camera;
use crate::graphics::Object;
use crate::math::Rect;
use crate::math::Vec2;
use crate::window::Error;

use sdl3::pixels::FColor;
use sdl3::render::{FPoint, Vertex};

/// `Option<FRect>: From<Rect>` does not exist, so the conversion goes through an
/// explicit local at every call site that needs one.
pub(super) fn to_f_rect(rect: impl Into<Rect>) -> sdl3::render::FRect {
    let rect: Rect = rect.into();
    rect.into()
}

/// Two triangles over four corners, in the winding `polygon_vertices` expects.
pub(super) const QUAD_INDICES: [i32; 6] = [0, 1, 2, 0, 2, 3];

/// Modulates nothing, so a textured draw passes the image through untouched.
pub(super) const WHITE: FColor = FColor {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

/// Turns `points` about `obj`'s centre, for the polygon methods to hand on.
pub(super) fn rotate_points(points: &[(f32, f32)], obj: &Object) -> Vec<(f32, f32)> {
    points
        .iter()
        .map(|&(x, y)| {
            let turned = obj.turn(Vec2::new(x, y));
            (turned.x, turned.y)
        })
        .collect()
}

/// Moves `points` out of world space and onto the screen through `camera`.
///
/// This runs after any turning, never before: a turn is a rotation about a
/// point, and doing it in world space keeps the angle the caller asked for,
/// while doing it here would scale the radius without the shape.
///
/// `camera` of `None` means the frame has no camera, and then the points are
/// screen coordinates already -- they come back copied, unchanged.
pub(super) fn project(
    points: &[(f32, f32)],
    camera: Option<Camera>,
    size: (u32, u32),
) -> Vec<(f32, f32)> {
    let camera = match camera {
        Some(camera) => camera,
        None => return points.to_vec(),
    };

    points
        .iter()
        .map(|&(x, y)| {
            let projected = camera.to_screen(Vec2::new(x, y), size);
            (projected.x, projected.y)
        })
        .collect()
}

/// Builds the vertex buffer for an indexed triangle list.
///
/// Shared by every `*_object` fill, which differ in what each vertex's
/// `tex_coord` says and whether the positions get turned. With `textured`, a
/// position is mapped onto the bounding box of the referenced vertices, so the
/// polygon covers the whole texture; without one, `tex_coord` stays `(0, 0)` and
/// the field is ignored. `turn_about` turns the positions only -- texture
/// coordinates are always taken from the polygon as given.
pub(super) fn polygon_vertices(
    points: &[(f32, f32)],
    indices: &[i32],
    color: FColor,
    textured: bool,
    turn_about: Option<&Object>,
) -> Result<Vec<Vertex>, Error> {
    // Both of these used to be `assert!`, which meant an index list the caller
    // got wrong killed the process. A library reports; it does not judge.
    if indices.len() % 3 != 0 {
        return Err(Error::IndicesNotTriples {
            count: indices.len(),
        });
    }

    let mut positions: Vec<(f32, f32)> = Vec::with_capacity(indices.len());
    for &index in indices {
        if index < 0 || index as usize >= points.len() {
            return Err(Error::IndexOutOfBounds {
                index,
                len: points.len(),
            });
        }
        positions.push(points[index as usize]);
    }

    if positions.is_empty() {
        return Ok(Vec::new());
    }

    // Only the vertices the indices reference take part in the box: a point that
    // nothing points at is not part of the polygon and must not stretch the
    // texture over it.
    let (min_x, min_y, w, h) = if textured {
        let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
        let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for &(x, y) in &positions {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        // A box with no width or height means every point sits on one line;
        // dividing by it would poison the coordinates with a NaN.
        (
            min_x,
            min_y,
            (max_x - min_x).max(f32::EPSILON),
            (max_y - min_y).max(f32::EPSILON),
        )
    } else {
        (0.0, 0.0, 1.0, 1.0)
    };

    let vertices = positions
        .into_iter()
        .map(|(x, y)| {
            // The turn happens here rather than on `positions` above, so the
            // bounding box above stays the polygon's own and the texture lands
            // where it would have without rotation.
            let turned = match turn_about {
                Some(obj) => obj.turn(Vec2::new(x, y)),
                None => Vec2::new(x, y),
            };

            Vertex {
                position: FPoint::new(turned.x, turned.y),
                color,
                tex_coord: if textured {
                    FPoint::new((x - min_x) / w, (y - min_y) / h)
                } else {
                    FPoint::new(0.0, 0.0)
                },
            }
        })
        .collect::<Vec<Vertex>>();
    Ok(vertices)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards the bug of making a camera the default: a camera at the origin
    /// with no zoom is *not* the identity, it centres the world on the window,
    /// so every drawing example ended up shifted by half a window.
    #[test]
    fn without_a_camera_points_stay_where_they_were() {
        let points = [(10.0, 20.0), (30.0, 40.0)];
        assert_eq!(project(&points, None, (800, 600)), points);
    }

    #[test]
    fn with_a_camera_the_looked_at_point_lands_in_the_middle() {
        let camera = Camera::new(Vec2::ZERO);
        let placed = project(&[(0.0, 0.0)], Some(camera), (800, 600));

        assert!((placed[0].0 - 400.0).abs() < 1e-4);
        assert!((placed[0].1 - 300.0).abs() < 1e-4);
    }

    #[test]
    fn the_zoom_scales_the_distance_already_turned() {
        // Turning first and projecting second keeps the radius the caller
        // asked for; the other order would scale it twice over.
        let camera = Camera::new(Vec2::ZERO).with_zoom(3.0);
        let turned = Vec2::new(100.0, 0.0).rotated(std::f32::consts::FRAC_PI_2);
        let placed = camera.to_screen(turned, (800, 600));

        // Turned to (0, 100) in world space, then tripled.
        assert!((placed.x - 400.0).abs() < 1e-3);
        assert!((placed.y - 600.0).abs() < 1e-3);
    }
}
