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

    /// Takes over a surface that was decoded somewhere else.
    ///
    /// This is the way in for pixels `frglib` never saw. Anything that
    /// produces an SDL surface and an example wants to draw it -- SDL3_ttf
    /// rendering a line of text, a procedural pattern, a gradient -- has
    /// nowhere else to go: [`Draw`](crate::Draw) takes an `Image`, and until
    /// now the only way to get one was a file on disk.
    ///
    /// It is also what keeps font code out of the library. Parsing an OTF is a
    /// large, security-sensitive job, and a game library has no business doing
    /// it on behalf of every user. So `frglib` leaves SDL3_ttf alone, and an
    /// example that wants text pulls it in as a dev-dependency and hands the
    /// result over here.
    ///
    /// The usual caveat of sdl3 types in a public signature applies here: the
    /// surface is the same one sdl3 uses, so it is tied to the sdl3 version this
    /// crate was built against.
    pub fn from_surface(surface: sdl3::surface::Surface<'static>) -> Self {
        Image { surface }
    }

    /// Width and height in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.surface.size()
    }
}
