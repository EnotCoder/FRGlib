//! Images loaded from disk.

use std::path::Path;

use crate::window::Error;

/// An image in memory, ready to be drawn onto a window.
///
/// Holds the decoded pixels on the CPU, not anything on the GPU, so it owns no
/// borrow of a window and can outlive one. Actually putting it on the GPU happens
/// per call to [`Draw::fill_textured_object`](crate::Draw::fill_textured_object).
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

    /// Cuts a rectangle out of the image into a new one.
    ///
    /// This is how a sheet gets taken apart. A tile map keeps many tiles side by
    /// side in one file, and a sprite sheet keeps every direction and every walk
    /// frame; either way each piece has to be drawable on its own.
    ///
    /// The region is copied, not referenced: the result shares nothing with the
    /// original, so the big sheet can be dropped straight after.
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Result<Self, Error> {
        // sdl3's `Rect` takes the corner as i32 and the size as u32, which is a
        // reasonable way round and an awkward one to remember.
        let src = sdl3::rect::Rect::new(x as i32, y as i32, width, height);

        // RGBA32 is RGBA at 8 bits per channel, which is what the loader gives
        // us for a PNG.
        let mut out =
            sdl3::surface::Surface::new(width, height, sdl3::pixels::PixelFormat::RGBA32)?;

        self.surface.blit(Some(src), &mut out, None)?;

        Ok(Image { surface: out })
    }

    /// Width and height in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.surface.size()
    }
}
