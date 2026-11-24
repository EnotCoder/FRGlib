//! The particles themselves: spawning, integrating and retiring them.
//!
//! Everything runs on the CPU. That keeps the system independent of both
//! windows, makes particles easy to inspect and test, and is plenty for the
//! few hundred sparks a 2D game usually has. Drawing is a separate concern in
//! [`draw`](super::draw).

use std::collections::vec_deque::{Iter, VecDeque};
use std::ops::Range;

use crate::graphics::Color;
use crate::math::{lerp, Ease, Vec2};

use super::emitter::{Emitter, ParticleShape};

/// One live particle.
///
/// `position`, `velocity`, `color` and `size` are updated every frame and are
/// meant to be read when drawing a particle in a custom way. The remaining
/// fields are private so a particle can only be created by a system, with the
/// birth values it needs to fade correctly.
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    /// Centre of the particle, in world units.
    pub position: Vec2,
    /// Current velocity, in units per second.
    pub velocity: Vec2,
    /// Current colour, interpolated between the emitter's two colours.
    pub color: Color,
    /// Current size across, interpolated down to `end_scale` times its birth
    /// value.
    pub size: f32,
    /// Shape chosen when the particle was born.
    pub shape: ParticleShape,
    /// Seconds since birth.
    pub age: f32,
    /// Total lifetime in seconds.
    pub life: f32,

    gravity: Vec2,
    drag: f32,
    birth_size: f32,
    end_scale: f32,
    birth_color: Color,
    end_color: Color,
}

impl Particle {
    /// How far through its life the particle is, from `0.0` to `1.0`.
    pub fn progress(&self) -> f32 {
        if self.life <= f32::EPSILON {
            1.0
        } else {
            (self.age / self.life).clamp(0.0, 1.0)
        }
    }

    /// Seconds left before the particle is retired.
    pub fn remaining(&self) -> f32 {
        (self.life - self.age).max(0.0)
    }

    /// Advances one step, returning `false` once the particle has expired.
    fn advance(&mut self, dt: f32) -> bool {
        self.age += dt;
        if self.age >= self.life {
            return false;
        }

        self.velocity += self.gravity * dt;
        // An exponential decay rather than `1.0 - drag * dt`: the result no
        // longer depends on how finely the frame time was sliced up.
        self.velocity = self.velocity * (1.0 - self.drag).powf(dt);
        self.position += self.velocity * dt;

        let t = self.progress();
        self.size = lerp(
            self.birth_size,
            self.birth_size * self.end_scale,
            t,
            Ease::Linear,
        );
        self.color = lerp(self.birth_color, self.end_color, t, Ease::Linear);
        true
    }
}

/// A pool of particles driven by an [`Emitter`].
///
/// Particles are kept in birth order. When the pool is full, a new particle
/// pushes the oldest one out, so a long-running emitter never grows without
/// bound and a burst still shows up in full.
#[derive(Debug, Clone)]
pub struct ParticleSystem {
    particles: VecDeque<Particle>,
    emitter: Emitter,
    capacity: usize,
    /// Fraction of a particle left over from earlier frames, so a rate of 30
    /// at 60 fps emits one every other frame instead of emitting none.
    pending: f32,
    rng: Rng,
}

impl ParticleSystem {
    /// An empty system that can hold up to `capacity` particles.
    ///
    /// A capacity of zero is treated as one: an emitter with no room at all
    /// would silently drop every particle.
    pub fn new(capacity: usize) -> Self {
        Self::with_emitter(capacity, Emitter::new())
    }

    /// An empty system with a ready-made emitter.
    pub fn with_emitter(capacity: usize, emitter: Emitter) -> Self {
        ParticleSystem {
            particles: VecDeque::with_capacity(capacity.max(1)),
            emitter,
            capacity: capacity.max(1),
            pending: 0.0,
            rng: Rng::new(0x9E37_79B9),
        }
    }

    /// Fixes the random sequence, so two systems emit the same sparks.
    ///
    /// Useful for tests and for screenshots that should look the same twice.
    pub fn seed(mut self, seed: u32) -> Self {
        self.rng = Rng::new(seed);
        self
    }

    /// The settings new particles are born with.
    pub fn emitter(&self) -> &Emitter {
        &self.emitter
    }

    /// Mutable settings: writing `origin` here moves a continuous emitter.
    pub fn emitter_mut(&mut self) -> &mut Emitter {
        &mut self.emitter
    }

    /// How many particles the pool holds.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// How many particles are alive right now.
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// Whether no particles are alive.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Live particles, in birth order from oldest to newest.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Particle> + '_ {
        self.particles.iter()
    }

    /// Throws away every live particle and the fraction carried between
    /// frames. New particles start from a clean slate.
    pub fn clear(&mut self) {
        self.particles.clear();
        self.pending = 0.0;
    }

    /// Emits `count` particles at the emitter's origin.
    pub fn emit(&mut self, count: usize) {
        let origin = self.emitter.origin;
        self.emit_at(count, origin);
    }

    /// Emits `count` particles at `origin`, whatever the emitter's current
    /// position is. This is the burst API: one call, a whole puff at once.
    ///
    /// A request larger than the pool is capped at its capacity: the oldest
    /// particles are recycled and the pool never grows past its limit.
    pub fn emit_at(&mut self, count: usize, origin: Vec2) {
        let count = count.min(self.capacity);
        if count == 0 {
            return;
        }

        // Recycling the oldest particles keeps the pool at capacity without
        // an allocation per spawn.
        let free = self.capacity - self.particles.len();
        if count > free {
            let drop = (count - free).min(self.particles.len());
            self.particles.drain(..drop);
        }

        for _ in 0..count {
            let particle = self.spawn(origin);
            self.particles.push_back(particle);
        }
    }

    /// Advances the simulation by `dt` seconds.
    ///
    /// Non-positive and non-finite steps are ignored: a paused game or a
    /// broken clock must not age or teleport the pool.
    pub fn update(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        let rate = if self.emitter.rate.is_finite() {
            self.emitter.rate.max(0.0)
        } else {
            0.0
        };
        self.pending += rate * dt;
        if !self.pending.is_finite() {
            self.pending = 0.0;
        }

        if self.pending >= 1.0 {
            // Saturating cast would overflow on a huge rate times a huge dt;
            // clamping to the pool size gives the same visible result.
            let count = (self.pending.floor() as usize).min(self.capacity);
            self.pending = self.pending.fract();
            self.emit(count);
        }

        self.particles.retain_mut(|particle| particle.advance(dt));
    }

    /// Creates one particle, sampling every range from the emitter once.
    fn spawn(&mut self, origin: Vec2) -> Particle {
        // A clone keeps the immutable settings out of the way of the mutable
        // random generator while the ranges are sampled.
        let emitter = self.emitter.clone();
        let angle = emitter.direction + (self.rng.next_f32() - 0.5) * emitter.spread;
        let speed = self.rng.range(&emitter.speed);
        let size = self.rng.range(&emitter.size).max(0.0);

        Particle {
            position: origin,
            velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
            color: emitter.color,
            size,
            shape: emitter.shape,
            age: 0.0,
            life: self.rng.range(&emitter.lifetime).max(f32::EPSILON),
            gravity: emitter.gravity,
            drag: emitter.drag.clamp(0.0, 1.0),
            birth_size: size,
            end_scale: emitter.end_scale.max(0.0),
            birth_color: emitter.color,
            end_color: emitter.end_color,
        }
    }
}

impl<'a> IntoIterator for &'a ParticleSystem {
    type Item = &'a Particle;
    type IntoIter = Iter<'a, Particle>;

    fn into_iter(self) -> Self::IntoIter {
        self.particles.iter()
    }
}

/// Tiny xorshift32, so particles need no `rand` dependency.
///
/// SDL-based examples already avoid `rand` for a handful of random values; the
/// engine can do the same, with a seed callers can fix for repeatable output.
#[derive(Debug, Clone)]
struct Rng {
    state: u32,
}

impl Rng {
    fn new(seed: u32) -> Self {
        Rng {
            state: if seed == 0 { 0x1_234_5678 } else { seed },
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform in `0.0..1.0`.
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform inside `range`, with reversed ends handled by the caller's
    /// builder. An empty range collapses to its single value.
    fn range(&mut self, range: &Range<f32>) -> f32 {
        let (lo, hi) = (range.start.min(range.end), range.start.max(range.end));
        if !lo.is_finite() || !hi.is_finite() || lo >= hi {
            return lo;
        }
        lo + self.next_f32() * (hi - lo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn still() -> ParticleSystem {
        ParticleSystem::with_emitter(
            64,
            Emitter::new()
                .speed(0.0, 0.0)
                .lifetime(1.0, 1.0)
                .size(10.0, 10.0),
        )
        .seed(7)
    }

    #[test]
    fn rate_emits_over_time_and_keeps_the_fraction() {
        let mut system = still();
        system.emitter_mut().rate = 10.0;

        system.update(0.05);
        assert_eq!(system.len(), 0, "half a particle is not a particle yet");
        system.update(0.05);
        assert_eq!(system.len(), 1);
        system.update(0.2);
        assert_eq!(system.len(), 3);
    }

    #[test]
    fn a_burst_spawns_all_particles_at_once() {
        let mut system = still();
        system.emit_at(5, Vec2::new(12.0, 34.0));

        assert_eq!(system.len(), 5);
        for particle in &system {
            assert_eq!(particle.position, Vec2::new(12.0, 34.0));
            assert_eq!(particle.velocity, Vec2::ZERO);
        }
    }

    #[test]
    fn particles_expire_after_their_lifetime() {
        let mut system = still();
        system.emit(1);

        system.update(0.5);
        assert_eq!(system.len(), 1);
        assert!((system.iter().next().unwrap().progress() - 0.5).abs() < 1e-6);

        system.update(0.51);
        assert!(system.is_empty());
    }

    #[test]
    fn a_full_pool_drops_the_oldest_particles() {
        let mut system =
            ParticleSystem::with_emitter(3, Emitter::new().speed(0.0, 0.0).lifetime(5.0, 5.0))
                .seed(7);
        system.emit_at(2, Vec2::new(1.0, 0.0));
        system.emit_at(2, Vec2::new(2.0, 0.0));

        let positions: Vec<f32> = system.iter().map(|p| p.position.x).collect();
        // One of the old pair makes room; the other is still the newest three.
        assert_eq!(positions, vec![1.0, 2.0, 2.0]);

        system.emit_at(1, Vec2::new(3.0, 0.0));
        let positions: Vec<f32> = system.iter().map(|p| p.position.x).collect();
        assert_eq!(positions, vec![2.0, 2.0, 3.0]);
        assert_eq!(system.len(), system.capacity());
    }

    #[test]
    fn an_oversized_burst_stays_within_capacity() {
        let mut system =
            ParticleSystem::with_emitter(4, Emitter::new().speed(0.0, 0.0).lifetime(5.0, 5.0));
        system.emit_at(100, Vec2::new(9.0, 0.0));

        assert_eq!(system.len(), 4);
        assert!(system.iter().all(|p| p.position == Vec2::new(9.0, 0.0)));
    }

    #[test]
    fn colour_and_size_fade_with_progress() {
        let mut system = still();
        system.emitter_mut().end_scale = 0.25;
        system.emitter_mut().color = Color::WHITE;
        system.emitter_mut().end_color = Color::rgba(0, 0, 0, 0);
        system.emit(1);

        system.update(0.5);
        let particle = system.iter().next().unwrap();
        assert!((particle.size - 6.25).abs() < 1e-4);
        assert_eq!(particle.color.r, 128);
        assert_eq!(particle.color.a, 128);
        assert!((particle.remaining() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gravity_changes_velocity_before_position() {
        let mut system = ParticleSystem::with_emitter(
            8,
            Emitter::new()
                .lifetime(5.0, 5.0)
                .speed(0.0, 0.0)
                .gravity(Vec2::new(0.0, 100.0)),
        );
        system.emit_at(1, Vec2::ZERO);

        system.update(0.1);
        let particle = system.iter().next().unwrap();
        assert!((particle.velocity.y - 10.0).abs() < 1e-4);
        assert!((particle.position.y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn drag_decay_is_independent_of_the_number_of_steps() {
        let settings = || {
            Emitter::new()
                .lifetime(5.0, 5.0)
                .speed(100.0, 100.0)
                .direction(0.0)
                .spread(0.0)
                .drag(0.5)
        };

        let mut once = ParticleSystem::with_emitter(8, settings());
        once.emit_at(1, Vec2::ZERO);
        once.update(1.0);

        let mut twice = ParticleSystem::with_emitter(8, settings());
        twice.emit_at(1, Vec2::ZERO);
        twice.update(0.5);
        twice.update(0.5);

        let a = once.iter().next().unwrap().velocity.x;
        let b = twice.iter().next().unwrap().velocity.x;
        assert!((a - b).abs() < 1e-4, "one step {a} vs two steps {b}");
    }

    #[test]
    fn bad_time_steps_do_nothing() {
        let mut system = still();
        system.emit(1);

        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            system.update(dt);
        }

        let particle = system.iter().next().unwrap();
        assert_eq!(particle.age, 0.0);
        assert_eq!(particle.position, Vec2::ZERO);
    }

    #[test]
    fn the_same_seed_repeats_the_same_sparks() {
        let settings = || {
            Emitter::new()
                .rate(10.0)
                .lifetime(0.5, 1.5)
                .speed(10.0, 90.0)
                .direction(0.3)
                .spread(1.2)
                .size(2.0, 8.0)
        };

        let mut a = ParticleSystem::with_emitter(32, settings()).seed(1234);
        let mut b = ParticleSystem::with_emitter(32, settings()).seed(1234);
        for _ in 0..10 {
            a.update(0.1);
            b.update(0.1);
        }

        let positions: Vec<[f32; 2]> = a.iter().map(|p| p.position.to_array()).collect();
        let other: Vec<[f32; 2]> = b.iter().map(|p| p.position.to_array()).collect();
        assert_eq!(positions, other);
        assert!(!positions.is_empty());
    }

    #[test]
    fn samples_stay_inside_their_ranges() {
        let mut rng = Rng::new(99);
        for _ in 0..1000 {
            let value = rng.range(&(3.0..7.0));
            assert!((3.0..7.0).contains(&value));
            assert_eq!(rng.range(&(2.0..2.0)), 2.0);
        }
    }
}
