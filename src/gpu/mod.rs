//! A `wgpu` window: SDL3 owns the platform side, `wgpu` owns the frames.
//!
//! [`WgpuWindow`] is a sibling of [`crate::Window`], not a mode of it. SDL3
//! still creates the window, pumps the events and gives us the native handle,
//! but no `SDL_Renderer` is attached: the swapchain, the command encoder and
//! the pipelines all belong to `wgpu`. That is why [`crate::Draw`],
//! [`crate::ui`] and [`crate::TileMap`] do not work here -- they are built on
//! `SDL_Renderer` and have no access to this window.
//!
//! Two levels are available on purpose:
//!
//! - ready-made pieces: [`Camera3D`], [`Mesh`], [`Renderer3D`] draw a cube, a
//!   grid or a line without touching `wgpu`;
//! - the raw API: [`Gpu`] exposes the `wgpu` instance, adapter, device and
//!   queue, and [`Frame`] exposes the encoder and both attachments, so custom
//!   pipelines can be recorded in the same frame.
//!
//! ```no_run
//! use frglib::{Color, Event};
//! use frglib::gpu::{Camera3D, Renderer3D, WgpuWindow};
//! use frglib::gpu::glam::Vec3;
//!
//! # fn main() -> Result<(), frglib::gpu::Error> {
//! let mut window = WgpuWindow::builder("FRGLib").size(800, 600).build()?;
//! let mut renderer = Renderer3D::new(window.gpu());
//! let camera = Camera3D::perspective(60.0, 0.1, 300.0)
//!     .position(Vec3::new(3.0, 2.0, 5.0))
//!     .target(Vec3::ZERO);
//!
//! loop {
//!     for event in window.poll()? {
//!         if matches!(event, Event::Quit | Event::CloseRequested) {
//!             return Ok(());
//!         }
//!     }
//!
//!     let Some(mut frame) = window.begin_frame()? else { continue };
//!     renderer.begin(&camera);
//!     renderer.draw_cube(Vec3::ZERO, Vec3::ONE, Color::RED);
//!     renderer.draw_grid(20, 1.0, Color::WHITE);
//!     renderer.finish(&mut frame);
//!     window.present(frame)?;
//! }
//! # }
//! ```

mod builder;
mod camera;
mod error;
mod mesh;
mod renderer;
mod window;

pub use builder::WgpuWindowBuilder;
pub use camera::Camera3D;
pub use error::Error;
pub use mesh::{GpuMesh, Mesh, Vertex};
pub use renderer::{Material, Renderer3D};
pub use window::{Frame, Gpu, WgpuWindow};

/// Depth format shared by the built-in pipelines and the frame's depth buffer.
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

// Re-exported so a caller never has to depend on the exact versions this crate
// picked. Both are part of the public API, so bumping either is a breaking
// change for anyone using the feature.
pub use glam;
pub use wgpu;

/// Converts an 8-bit sRGB colour into the linear RGBA the shaders work in.
pub(crate) fn color_to_linear(color: crate::graphics::Color) -> [f32; 4] {
    [
        srgb_to_linear(color.r),
        srgb_to_linear(color.g),
        srgb_to_linear(color.b),
        color.a as f32 / 255.0,
    ]
}

/// Same colour, in the shape `wgpu` wants for a clear value.
///
/// `wgpu` interprets clear values in linear space when the target is sRGB, so
/// the conversion is the same one the vertex colours go through.
pub(crate) fn color_to_wgpu(color: crate::graphics::Color) -> wgpu::Color {
    let [r, g, b, a] = color_to_linear(color);
    wgpu::Color {
        r: r as f64,
        g: g as f64,
        b: b as f64,
        a: a as f64,
    }
}

fn srgb_to_linear(channel: u8) -> f32 {
    let channel = channel as f32 / 255.0;
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_and_white_survive_the_conversion() {
        let black = color_to_linear(crate::graphics::Color::BLACK);
        let white = color_to_linear(crate::graphics::Color::WHITE);

        assert_eq!(black, [0.0, 0.0, 0.0, 1.0]);
        assert!((white[0] - 1.0).abs() < 1e-6);
        assert!((white[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mid_grey_becomes_less_than_half_in_linear_space() {
        let grey = color_to_linear(crate::graphics::Color::rgb(128, 128, 128));

        assert!(grey[0] < 0.5, "sRGB 128 is about 0.216 linear");
        assert!(grey[0] > 0.2);
    }

    #[test]
    fn alpha_is_not_gamma_corrected() {
        let color = color_to_linear(crate::graphics::Color::rgba(0, 0, 0, 128));

        assert!((color[3] - 128.0 / 255.0).abs() < 1e-6);
    }
}
