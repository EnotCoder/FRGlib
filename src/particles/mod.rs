//! Particles: a small emitter with drawing for both window flavours.
//!
//! A [`ParticleSystem`] owns a fixed pool of [`Particle`]s. The [`Emitter`]
//! decides how they are born -- how fast, in which direction, for how long and
//! with which colour -- while `update(dt)` moves them, fades them and retires
//! the ones whose time is up. Nothing here depends on SDL or `wgpu`, so the
//! same system can be drawn through [`ParticleSystem::draw`] on an SDL window
//! or [`ParticleSystem::draw_gpu`] on a GPU window.
//!
//! ```
//! use frglib::{Color, Emitter, ParticleShape, ParticleSystem, Vec2};
//!
//! let mut sparks = ParticleSystem::with_emitter(
//!     512,
//!     Emitter::new()
//!         .rate(40.0)
//!         .at(Vec2::new(400.0, 300.0))
//!         .direction(-std::f32::consts::FRAC_PI_2)
//!         .spread(0.7)
//!         .speed(80.0, 200.0)
//!         .lifetime(0.4, 0.9)
//!         .colors(Color::WHITE, Color::rgba(255, 80, 40, 0))
//!         .shape(ParticleShape::Circle),
//! );
//!
//! sparks.update(1.0 / 60.0);
//! sparks.emit_at(30, Vec2::new(100.0, 100.0)); // a burst, independent of rate
//! assert!(!sparks.is_empty());
//! ```

mod draw;
mod emitter;
mod system;

pub use emitter::{Emitter, ParticleShape};
pub use system::{Particle, ParticleSystem};
