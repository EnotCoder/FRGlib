//! Builder for [`WgpuWindow`].

use crate::gpu::{Error, WgpuWindow};
use crate::graphics::Color;

/// Configures and creates a [`WgpuWindow`].
///
/// Deliberately smaller than [`crate::WindowBuilder`]: the SDL renderer
/// options (pixel art, Vulkan instance flags) do not apply to a wgpu
/// swapchain, and the GPU picks its own presentation mode.
///
/// ```no_run
/// # fn main() -> Result<(), frglib::gpu::Error> {
/// let window = frglib::gpu::WgpuWindow::builder("FRGLib")
///     .size(800, 600)
///     .resizable(true)
///     .background(frglib::Color::BLACK)
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct WgpuWindowBuilder {
    title: String,
    width: u32,
    height: u32,
    resizable: bool,
    background: Color,
}

impl WgpuWindowBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 800,
            height: 600,
            resizable: true,
            background: Color::BLACK,
        }
    }

    /// Window size in logical pixels. Defaults to 800x600.
    ///
    /// The GPU surface follows the physical size, which is this value times
    /// the display scale on a HiDPI screen.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Whether the user may resize the window. Defaults to true.
    ///
    /// The surface and the depth buffer are reconfigured on the next frame
    /// when the size changes.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Colour frames are cleared to. Defaults to [`Color::BLACK`].
    pub fn background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    /// Opens the window and initialises the GPU.
    pub fn build(self) -> Result<WgpuWindow, Error> {
        WgpuWindow::create(
            self.title,
            self.width,
            self.height,
            self.resizable,
            self.background,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_builder_has_the_default_size_and_background() {
        let builder = WgpuWindowBuilder::new("demo");

        assert_eq!(builder.width, 800);
        assert_eq!(builder.height, 600);
        assert!(builder.resizable);
        assert_eq!(builder.background, Color::BLACK);
    }

    #[test]
    fn every_option_can_be_changed() {
        let builder = WgpuWindowBuilder::new("demo")
            .size(1280, 720)
            .resizable(false)
            .background(Color::RED);

        assert_eq!(builder.width, 1280);
        assert_eq!(builder.height, 720);
        assert!(!builder.resizable);
        assert_eq!(builder.background, Color::RED);
    }
}
