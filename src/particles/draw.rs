//! Drawing a particle pool through either window flavour.
//!
//! The simulation itself knows nothing about SDL or `wgpu`; these two methods
//! are the bridge. Both draw in whatever coordinate space the caller's draw
//! object is currently in, so setting a camera on the window or renderer moves
//! the particles together with the rest of the world.

use crate::graphics::Object;
use crate::window::{Draw, Error};

#[cfg(feature = "wgpu")]
use crate::math::Rect;

use super::{ParticleShape, ParticleSystem};

/// Segments in the polygon used for a round particle on the SDL side. Small
/// enough to be cheap for a burst, round enough at a few pixels across.
const SEGMENTS: usize = 12;

impl ParticleSystem {
    /// Queues every live particle on an SDL window.
    ///
    /// Call after setting the camera, if the frame has one. Particles with no
    /// visible size or fully transparent colour are skipped.
    pub fn draw(&self, draw: &mut Draw<'_>) -> Result<(), Error> {
        let fan = triangle_fan();

        for particle in self.iter() {
            if particle.size <= 0.0 || particle.color.a == 0 {
                continue;
            }

            match particle.shape {
                ParticleShape::Square => {
                    let obj = Object::from_xywh(
                        particle.position.x,
                        particle.position.y,
                        particle.size,
                        particle.size,
                    );
                    draw.fill_object(&obj, particle.color)?;
                }
                ParticleShape::Circle => {
                    let radius = particle.size * 0.5;
                    let points: Vec<(f32, f32)> = (0..SEGMENTS)
                        .map(|index| {
                            let angle = std::f32::consts::TAU * index as f32 / SEGMENTS as f32;
                            (
                                particle.position.x + angle.cos() * radius,
                                particle.position.y + angle.sin() * radius,
                            )
                        })
                        .collect();
                    draw.fill_polygon(&points, &fan, particle.color)?;
                }
            }
        }

        Ok(())
    }

    /// Queues every live particle on a `wgpu` 2D renderer.
    ///
    /// Works both for a full 2D frame and for an overlay on top of a 3D scene.
    /// Uses the renderer's current camera, if it has one.
    #[cfg(feature = "wgpu")]
    pub fn draw_gpu(&self, ui: &mut crate::gpu::Renderer2D) {
        for particle in self.iter() {
            if particle.size <= 0.0 || particle.color.a == 0 {
                continue;
            }

            let half = particle.size * 0.5;
            match particle.shape {
                ParticleShape::Square => ui.fill_rect(
                    Rect::new(
                        particle.position.x - half,
                        particle.position.y - half,
                        particle.size,
                        particle.size,
                    ),
                    particle.color,
                ),
                ParticleShape::Circle => ui.fill_circle(particle.position, half, particle.color),
            }
        }
    }
}

/// Indices turning the `SEGMENTS` points of a circle into a triangle fan.
fn triangle_fan() -> [i32; (SEGMENTS - 2) * 3] {
    let mut indices = [0i32; (SEGMENTS - 2) * 3];
    for index in 0..SEGMENTS - 2 {
        indices[index * 3] = 0;
        indices[index * 3 + 1] = index as i32 + 1;
        indices[index * 3 + 2] = index as i32 + 2;
    }
    indices
}
