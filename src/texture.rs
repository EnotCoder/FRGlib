//! Images living on the GPU, uploaded once.
//!
//! Only present with the `cached-textures` feature -- see [`Texture`] for why
//! that is not the default.

use crate::image::Image;
use crate::window::{Error, Window};

/// A handle to an image already on the GPU.
///
/// Obtain one with [`Window::upload`] and hand it to the drawing methods. This
/// is the fast path: without it, every textured draw decodes and uploads the
/// image again, which for an 800x600 RGBA costs about 1.9 MB per call.
///
/// The handle is a plain index, so it is `Copy` and costs nothing to keep around.
/// The pixels belong to the window that uploaded them and go away with it.
///
/// # Why this is behind a feature
///
/// `sdl3` ties its texture type to the creator that made it, so a texture
/// cannot outlive the canvas. Holding one across frames would make [`Window`]
/// refer to its own field, which safe Rust cannot express. The way past it is
/// `sdl3`'s `unsafe_textures`, which gives up the link and admits that using a
/// texture with the wrong canvas is undefined behaviour.
///
/// We never do that: the textures are stored inside the window that made them,
/// the field is declared so that they are dropped while its renderer is still
/// alive, and drawing resolves the handle back to that one canvas. The risk is
/// opt-in, and the crate stays sound without it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Texture {
    slot: usize,
}

/// The window's textures, indexed by [`Texture::slot`].
///
/// Separate from the handle on purpose. If the handle owned the pixels, it
/// would borrow the window for as long as it lived, and `poll` and `frame_with`
/// want `&mut Window` -- so a texture could never be held across a frame, which
/// is the entire point.
pub(crate) struct Store {
    textures: Vec<sdl3::render::Texture>,
}

impl Store {
    pub(crate) fn new() -> Self {
        Store {
            textures: Vec::new(),
        }
    }

    /// Takes ownership of a texture and returns the handle to it.
    pub(crate) fn push(&mut self, texture: sdl3::render::Texture) -> Texture {
        self.textures.push(texture);
        Texture {
            slot: self.textures.len() - 1,
        }
    }

    pub(crate) fn get(&self, texture: Texture) -> &sdl3::render::Texture {
        &self.textures[texture.slot]
    }
}

impl Window {
    /// Uploads `image` to the GPU once and hands back a handle to it.
    ///
    /// This does the work that
    /// [`fill_textured_object`](crate::Draw::fill_textured_object) otherwise
    /// repeats on every call, so drawing one image many times costs a single
    /// upload rather than one per object per frame.
    ///
    /// The pixels live as long as the window does, so this cannot outlive the
    /// renderer they were uploaded to.
    pub fn upload(&mut self, image: &Image) -> Result<Texture, Error> {
        // `texture_creator` borrows the canvas and the texture borrows the
        // creator. `unsafe_textures` is what lets the result be moved into the
        // store; the field order in `Window` is what keeps the destruction
        // safe.
        let creator = self.canvas.texture_creator();
        let texture = creator.create_texture_from_surface(&image.surface)?;

        Ok(self.textures.push(texture))
    }
}
