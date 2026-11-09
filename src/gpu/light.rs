//! Lights for the built-in 3D shader.
//!
//! [`Renderer3D`](super::Renderer3D) keeps a list of lights that every
//! [`Material::Lit`](super::Material::Lit) draw uses. The built-in shader
//! supports up to [`MAX_LIGHTS`] of them, mixing directional and point lights
//! freely.

use glam::Vec3;

use crate::graphics::Color;

/// How many lights the built-in shader can use at once.
pub const MAX_LIGHTS: usize = 8;

/// What kind of source a [`Light`] is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightKind {
    /// Parallel rays from a far-away source, such as the sun.
    Directional {
        /// The direction the light travels, from the source towards the scene.
        direction: Vec3,
    },
    /// A point source that falls off with distance.
    Point {
        /// Where the light sits in world space.
        position: Vec3,
        /// Distance at which the light has fallen off substantially.
        range: f32,
    },
}

/// One light in a scene.
///
/// Colours are ordinary sRGB [`Color`] values; the renderer converts them to
/// linear space together with the rest of the scene. `intensity` scales the
/// contribution, so `1.0` is the neutral value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    /// Directional or point.
    pub kind: LightKind,
    /// Light colour.
    pub color: Color,
    /// Multiplier applied to the colour.
    pub intensity: f32,
}

impl Light {
    /// A directional light travelling in `direction`.
    ///
    /// The default intensity is `1.0`. A zero direction points straight down,
    /// which keeps a malformed light from producing undefined values.
    pub fn directional(direction: Vec3, color: Color) -> Self {
        Light {
            kind: LightKind::Directional { direction },
            color,
            intensity: 1.0,
        }
    }

    /// A point light at `position` with a default range of 10 world units.
    pub fn point(position: Vec3, color: Color) -> Self {
        Light {
            kind: LightKind::Point {
                position,
                range: 10.0,
            },
            color,
            intensity: 1.0,
        }
    }

    /// Changes the intensity, clamping it to a non-negative value.
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.max(0.0);
        self
    }

    /// Changes the range of a point light. Directional lights ignore it.
    pub fn with_range(mut self, range: f32) -> Self {
        if let LightKind::Point { range: value, .. } = &mut self.kind {
            *value = range.max(0.0);
        }
        self
    }

    /// The direction a directional light travels in, if this is one.
    pub fn direction(&self) -> Option<Vec3> {
        match self.kind {
            LightKind::Directional { direction } => Some(direction),
            LightKind::Point { .. } => None,
        }
    }

    /// The position of a point light, if this is one.
    pub fn position(&self) -> Option<Vec3> {
        match self.kind {
            LightKind::Point { position, .. } => Some(position),
            LightKind::Directional { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_start_at_full_intensity() {
        let directional = Light::directional(Vec3::NEG_Y, Color::WHITE);
        let point = Light::point(Vec3::ZERO, Color::RED);

        assert_eq!(directional.intensity, 1.0);
        assert_eq!(point.intensity, 1.0);
    }

    #[test]
    fn intensity_and_range_are_clamped() {
        let light = Light::point(Vec3::ZERO, Color::WHITE)
            .with_intensity(-2.0)
            .with_range(-5.0);

        assert_eq!(light.intensity, 0.0);
        assert!(matches!(light.kind, LightKind::Point { range: 0.0, .. }));
    }

    #[test]
    fn accessors_only_answer_for_their_own_kind() {
        let directional = Light::directional(Vec3::NEG_Y, Color::WHITE);
        let point = Light::point(Vec3::ZERO, Color::WHITE);

        assert_eq!(directional.direction(), Some(Vec3::NEG_Y));
        assert_eq!(directional.position(), None);
        assert_eq!(point.position(), Some(Vec3::ZERO));
        assert_eq!(point.direction(), None);
    }
}
