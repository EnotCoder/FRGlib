//! Procedural noise.
//!
//! [`Perlin`] generates smooth, repeatable values from coordinates and a seed.
//! It is independent of the renderer and never allocates while sampling.

pub mod perlin;

pub use perlin::Perlin;
