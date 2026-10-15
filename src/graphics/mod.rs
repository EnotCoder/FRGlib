//! What gets drawn: colours, images, objects, the camera, and the GPU side.
//!
//! One directory per concern rather than one file per type, so the tree says
//! what the crate does rather than listing every struct it owns. `window` is
//! the other half: it owns the canvas these are drawn onto.

pub mod camera;
pub mod color;
pub mod image;
pub mod object;
#[cfg(feature = "cached-textures")]
pub mod texture;

pub use camera::Camera;
pub use color::Color;
pub use image::Image;
pub use object::Object;
#[cfg(feature = "cached-textures")]
pub use texture::Texture;
