//! Builder for [`Window`].

use crate::graphics::Color;
use crate::window::{Error, Window};
use sdl3::render::ScaleMode;

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
    // Stored as the SDL scale mode rather than the boolean the caller gives us:
    // applying it at the three places a texture is created is then a single
    // unconditional call instead of a branch in each.
    scale: ScaleMode,
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
            scale: ScaleMode::Linear,
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

    /// Whether textures keep hard pixel edges when scaled. Defaults to false.
    ///
    /// Off, SDL interpolates between texels: the right default for anything
    /// drawn from photographs or vectors, and the reason small pixel art comes
    /// out looking smeared. On, each texel is either kept or dropped whole, so
    /// a sprite drawn at two or four times its own size stays crisp.
    ///
    /// This is a per-window switch, applied to every texture drawn on it. To mix
    /// pixel art with a smooth image in one window, draw the smooth one into an
    /// [`Image`] and scale it yourself first.
    ///
    /// Nearest sampling only looks right at a whole-number scale. At
    /// `Camera::zoom` of 1.5 some texels land doubled and others not, which is
    /// inherent to the mode rather than a fault in it -- pixel art is usually
    /// zoomed by whole numbers for exactly that reason.
    ///
    /// ```no_run
    /// # fn main() -> Result<(), frglib::Error> {
    /// let window = frglib::Window::builder("game")
    ///     .pixel_art(true)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn pixel_art(mut self, pixel_art: bool) -> Self {
        self.scale = if pixel_art {
            ScaleMode::Nearest
        } else {
            ScaleMode::Linear
        };
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
            self.scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smooth unless the caller asks otherwise, so nothing changes for anyone
    /// who never mentions it.
    #[test]
    fn scaling_is_smooth_by_default() {
        assert_eq!(WindowBuilder::new("demo").scale, ScaleMode::Linear);
    }

    #[test]
    fn pixel_art_picks_the_nearest_texel() {
        assert_eq!(
            WindowBuilder::new("demo").pixel_art(true).scale,
            ScaleMode::Nearest
        );
    }

    /// The switch takes a boolean, so it has to be able to say no as well.
    #[test]
    fn pixel_art_can_be_turned_back_off() {
        let builder = WindowBuilder::new("demo").pixel_art(true).pixel_art(false);

        assert_eq!(builder.scale, ScaleMode::Linear);
    }

    #[test]
    fn pixel_art_leaves_the_rest_of_the_builder_alone() {
        let builder = WindowBuilder::new("demo").pixel_art(true);

        assert_eq!(builder.width, 800);
        assert_eq!(builder.height, 600);
        assert!(builder.resizable);
        assert!(!builder.vulkan);
    }
}
