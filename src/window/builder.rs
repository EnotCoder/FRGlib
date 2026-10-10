//! Builder for [`Window`].

use crate::color::Color;
use crate::window::{Error, Window};

/// Configures and creates a [`Window`].
///
/// Replaces the five positional arguments `Window::new` used to take, which were
/// easy to get wrong at the call site -- booleans in a row invite the compiler
/// to catch nothing when they are swapped.
///
/// ```no_run
/// # fn main() -> Result<(), frglib::Error> {
/// let window = frglib::Window::builder("FRGLib")
///     .size(800, 600)
///     .resizable(true)
///     .vulkan(false)
///     .background(frglib::Color::GREEN)
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct WindowBuilder {
    title: String,
    width: u32,
    height: u32,
    resizable: bool,
    vulkan: bool,
    background: Color,
}

impl WindowBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 800,
            height: 600,
            resizable: true,
            vulkan: false,
            background: Color::GREEN,
        }
    }

    /// Drawable size in pixels. Defaults to 800x600.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Whether the user may resize the window. Defaults to true.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Whether the window gets `SDL_WINDOW_VULKAN`. Defaults to false.
    pub fn vulkan(mut self, vulkan: bool) -> Self {
        self.vulkan = vulkan;
        self
    }

    /// Colour [`Window::frame`] fills the window with. Defaults to
    /// [`Color::GREEN`].
    pub fn background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    /// Opens the window.
    pub fn build(self) -> Result<Window, Error> {
        Window::create(
            self.title,
            self.width,
            self.height,
            self.resizable,
            self.vulkan,
            self.background,
        )
    }
}
