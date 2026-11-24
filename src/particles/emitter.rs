//! How new particles are born.
//!
//! [`Emitter`] is plain configuration: it owns no particles and advances no
//! time. [`ParticleSystem`](super::ParticleSystem) reads it when it spawns, so
//! the settings can be changed at any point in a frame -- a moving emitter is
//! just a new `origin` written before the next `update`.

use std::f32::consts::TAU;
use std::ops::Range;

use crate::graphics::Color;
use crate::math::Vec2;

/// The shape a particle is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleShape {
    /// An axis-aligned square of `particle.size` across.
    Square,
    /// A circle of `particle.size` across.
    Circle,
}

/// Settings for the particles a [`ParticleSystem`](super::ParticleSystem)
/// spawns.
///
/// All ranges are sampled uniformly for every new particle. `direction` and
/// `spread` are in radians; sizes and speeds are in world units, so they are
/// pixels until a camera is set on the drawing side.
#[derive(Debug, Clone)]
pub struct Emitter {
    /// Where particles are born when [`emit`](super::ParticleSystem::emit) is
    /// called. [`emit_at`](super::ParticleSystem::emit_at) overrides it.
    pub origin: Vec2,
    /// New particles per second. `0.0` means no automatic emission; call
    /// `emit` or `emit_at` by hand for bursts.
    pub rate: f32,
    /// How long a particle lives, in seconds.
    pub lifetime: Range<f32>,
    /// Initial speed, in units per second.
    pub speed: Range<f32>,
    /// Centre of the cone particles fly into. `-PI / 2` points up the screen.
    pub direction: f32,
    /// Full width of the cone, in radians. `TAU` means every direction.
    pub spread: f32,
    /// Constant acceleration, such as gravity, in units per second squared.
    pub gravity: Vec2,
    /// Fraction of speed lost each second, from `0.0` to `1.0`.
    pub drag: f32,
    /// Size at birth, in units across.
    pub size: Range<f32>,
    /// Size at death as a fraction of the size at birth. `0.0` shrinks a
    /// particle to nothing, `1.0` keeps its size.
    pub end_scale: f32,
    /// Colour at birth.
    pub color: Color,
    /// Colour at death. Fading out is just an alpha of zero here.
    pub end_color: Color,
    /// Shape used by [`draw`](super::ParticleSystem::draw) and
    /// [`draw_gpu`](super::ParticleSystem::draw_gpu).
    pub shape: ParticleShape,
}

impl Default for Emitter {
    fn default() -> Self {
        Emitter {
            origin: Vec2::ZERO,
            rate: 0.0,
            lifetime: 0.5..1.0,
            speed: 40.0..120.0,
            direction: 0.0,
            spread: TAU,
            gravity: Vec2::ZERO,
            drag: 0.0,
            size: 3.0..6.0,
            end_scale: 0.0,
            color: Color::WHITE,
            end_color: Color::rgba(255, 255, 255, 0),
            shape: ParticleShape::Circle,
        }
    }
}

impl Emitter {
    /// An emitter with no automatic emission and sensible defaults elsewhere.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the spawn point.
    pub fn at(mut self, origin: Vec2) -> Self {
        self.origin = origin;
        self
    }

    /// Sets how many particles spawn per second.
    pub fn rate(mut self, rate: f32) -> Self {
        self.rate = rate;
        self
    }

    /// Sets the range of lifetimes, in seconds.
    pub fn lifetime(mut self, min: f32, max: f32) -> Self {
        self.lifetime = span(min, max);
        self
    }

    /// Sets the range of initial speeds.
    pub fn speed(mut self, min: f32, max: f32) -> Self {
        self.speed = span(min, max);
        self
    }

    /// Points the emission cone in `radians`.
    pub fn direction(mut self, radians: f32) -> Self {
        self.direction = radians;
        self
    }

    /// Sets the full width of the emission cone, in radians.
    pub fn spread(mut self, radians: f32) -> Self {
        self.spread = radians;
        self
    }

    /// Sets the constant acceleration applied to every new particle.
    pub fn gravity(mut self, gravity: Vec2) -> Self {
        self.gravity = gravity;
        self
    }

    /// Sets how much speed a particle loses each second.
    pub fn drag(mut self, drag: f32) -> Self {
        self.drag = drag;
        self
    }

    /// Sets the range of sizes at birth, in units across.
    pub fn size(mut self, min: f32, max: f32) -> Self {
        self.size = span(min, max);
        self
    }

    /// Sets the size at death as a fraction of the size at birth.
    pub fn end_scale(mut self, scale: f32) -> Self {
        self.end_scale = scale;
        self
    }

    /// Sets the birth and death colours.
    pub fn colors(mut self, start: Color, end: Color) -> Self {
        self.color = start;
        self.end_color = end;
        self
    }

    /// Sets the shape particles are drawn with.
    pub fn shape(mut self, shape: ParticleShape) -> Self {
        self.shape = shape;
        self
    }
}

/// A range that stays ordered even if the caller passes its ends the other way
/// round. Sampling an empty range then simply yields its one possible value.
fn span(min: f32, max: f32) -> Range<f32> {
    min.min(max)..min.max(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_emit_nothing_but_are_drawable() {
        let emitter = Emitter::new();
        assert_eq!(emitter.rate, 0.0);
        assert_eq!(emitter.shape, ParticleShape::Circle);
        assert_eq!(emitter.color, Color::WHITE);
    }

    #[test]
    fn builders_set_every_field() {
        let emitter = Emitter::new()
            .at(Vec2::new(1.0, 2.0))
            .rate(12.0)
            .lifetime(0.2, 0.8)
            .speed(10.0, 20.0)
            .direction(1.0)
            .spread(2.0)
            .gravity(Vec2::new(0.0, 9.0))
            .drag(0.5)
            .size(2.0, 4.0)
            .end_scale(0.25)
            .colors(Color::RED, Color::BLUE)
            .shape(ParticleShape::Square);

        assert_eq!(emitter.origin, Vec2::new(1.0, 2.0));
        assert_eq!(emitter.rate, 12.0);
        assert_eq!(emitter.lifetime, 0.2..0.8);
        assert_eq!(emitter.speed, 10.0..20.0);
        assert_eq!(emitter.direction, 1.0);
        assert_eq!(emitter.spread, 2.0);
        assert_eq!(emitter.gravity, Vec2::new(0.0, 9.0));
        assert_eq!(emitter.drag, 0.5);
        assert_eq!(emitter.size, 2.0..4.0);
        assert_eq!(emitter.end_scale, 0.25);
        assert_eq!(emitter.color, Color::RED);
        assert_eq!(emitter.end_color, Color::BLUE);
        assert_eq!(emitter.shape, ParticleShape::Square);
    }

    #[test]
    fn reversed_ranges_are_put_in_order() {
        let emitter = Emitter::new().lifetime(1.0, 0.25).speed(30.0, 10.0);
        assert_eq!(emitter.lifetime, 0.25..1.0);
        assert_eq!(emitter.speed, 10.0..30.0);
    }
}
