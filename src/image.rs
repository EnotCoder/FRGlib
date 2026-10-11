//! Images loaded from disk.

use std::path::Path;

use crate::window::Error;

/// An image in memory, ready to be drawn onto a window.
///
/// Holds the decoded pixels on the CPU, not anything on the GPU, so it owns no
/// borrow of a window and can outlive one. Actually putting it on the GPU happens
/// per call to [`Draw::fill_texture`](crate::Draw::fill_texture).
pub struct Image {
    pub(crate) surface: sdl3::surface::Surface<'static>,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (w, h) = self.size();
        write!(f, "Image {{ {w}x{h} }}")
    }
}

impl Image {
    /// Loads an image from a file.
    ///
    /// SDL detects PNG and BMP on its own. Other formats need SDL built with
    /// the matching image library.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Image {
            surface: sdl3::surface::Surface::load_surface(path)?,
        })
    }

    /// Width and height in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.surface.size()
    }
}
