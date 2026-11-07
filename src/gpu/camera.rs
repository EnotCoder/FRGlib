use glam::camera::rh::proj::directx::perspective;
use glam::camera::rh::view::look_at_mat4;
use glam::{Mat4, Vec3};

use crate::math::deg_to_rad;

/// A perspective camera for the [`WgpuWindow`](super::WgpuWindow) world.
///
/// Unlike [`crate::Camera`], which is a 2D centre-and-zoom pair, this one has a
/// position and a target in three dimensions. It is right-handed and looks down
/// `-Z` in view space, which is what the built-in shaders expect.
///
/// ```no_run
/// use frglib::gpu::Camera3D;
/// use frglib::gpu::glam::Vec3;
///
/// let camera = Camera3D::perspective(60.0, 0.1, 500.0)
///     .position(Vec3::new(3.0, 2.0, 5.0))
///     .target(Vec3::ZERO);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3D {
    /// Where the camera is.
    pub position: Vec3,
    /// What it looks at.
    pub target: Vec3,
    /// Which way is up. [`Vec3::Y`] unless the camera is rolled.
    pub up: Vec3,
    /// Vertical field of view, in degrees.
    pub fov_y: f32,
    /// Near clipping distance. Must be greater than zero.
    pub z_near: f32,
    /// Far clipping distance. Must be greater than `z_near`.
    pub z_far: f32,
}

impl Camera3D {
    /// A perspective camera looking from the origin at `-Z`.
    pub fn perspective(fov_y: f32, z_near: f32, z_far: f32) -> Self {
        Camera3D {
            position: Vec3::ZERO,
            target: Vec3::NEG_Z,
            up: Vec3::Y,
            fov_y,
            z_near,
            z_far,
        }
    }

    /// Moves the camera to `position`, keeping the target and orientation.
    #[must_use]
    pub fn position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    /// Points the camera at `target`.
    #[must_use]
    pub fn target(mut self, target: Vec3) -> Self {
        self.target = target;
        self
    }

    /// Changes which way is up.
    #[must_use]
    pub fn up(mut self, up: Vec3) -> Self {
        self.up = up;
        self
    }

    /// Changes the vertical field of view, in degrees.
    #[must_use]
    pub fn fov_y(mut self, fov_y: f32) -> Self {
        self.fov_y = fov_y;
        self
    }

    /// Changes the clipping planes.
    #[must_use]
    pub fn clip(mut self, z_near: f32, z_far: f32) -> Self {
        self.z_near = z_near;
        self.z_far = z_far;
        self
    }

    /// Places the camera on a sphere around `centre`.
    ///
    /// `yaw` turns around the vertical axis and `pitch` lifts the camera above
    /// the horizon; both are in radians. A classic orbit camera for inspecting
    /// a model.
    #[must_use]
    pub fn orbit(centre: Vec3, radius: f32, yaw: f32, pitch: f32) -> Self {
        let (sin_yaw, cos_yaw) = yaw.sin_cos();
        let (sin_pitch, cos_pitch) = pitch.sin_cos();
        let offset = Vec3::new(
            radius * cos_pitch * sin_yaw,
            radius * sin_pitch,
            radius * cos_pitch * cos_yaw,
        );

        Camera3D::perspective(60.0, 0.1, 500.0)
            .position(centre + offset)
            .target(centre)
    }

    /// The view matrix.
    pub fn view(&self) -> Mat4 {
        look_at_mat4(self.position, self.target, self.up)
    }

    /// The projection matrix for an `aspect` of width / height.
    ///
    /// A zero or negative aspect is a window with no width yet; it is clamped
    /// to a tiny positive number so the matrix stays finite for that one frame.
    pub fn projection(&self, aspect: f32) -> Mat4 {
        perspective(
            deg_to_rad(self.fov_y),
            aspect.max(f32::EPSILON),
            self.z_near,
            self.z_far,
        )
    }

    /// The combined matrix the vertex shader multiplies positions by.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_camera_looks_down_negative_z() {
        let camera = Camera3D::perspective(60.0, 0.1, 100.0);
        assert_eq!(camera.position, Vec3::ZERO);
        assert_eq!(camera.target, Vec3::NEG_Z);
    }

    #[test]
    fn a_point_on_the_view_axis_lands_in_the_middle_of_the_screen() {
        let camera = Camera3D::perspective(60.0, 0.1, 100.0)
            .position(Vec3::new(0.0, 0.0, 5.0))
            .target(Vec3::ZERO);

        let clip = camera.view_projection(1.0) * Vec3::ZERO.extend(1.0);

        assert!(clip.x.abs() < 1e-5, "x should be centred, got {}", clip.x);
        assert!(clip.y.abs() < 1e-5, "y should be centred, got {}", clip.y);
        assert!(clip.w > 0.0, "the origin should be in front of the camera");
    }

    #[test]
    fn orbit_keeps_the_requested_distance() {
        let centre = Vec3::new(1.0, 2.0, 3.0);
        let camera = Camera3D::orbit(centre, 8.0, 0.7, 0.4);

        assert!((camera.position.distance(centre) - 8.0).abs() < 1e-4);
        assert_eq!(camera.target, centre);
    }

    #[test]
    fn a_zero_aspect_still_produces_a_finite_projection() {
        let camera = Camera3D::perspective(60.0, 0.1, 100.0);
        let matrix = camera.projection(0.0);

        assert!(matrix.to_cols_array().iter().all(|value| value.is_finite()));
    }
}
