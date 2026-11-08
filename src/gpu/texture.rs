//! A GPU texture that any renderer can draw.
//!
//! [`Renderer2D`](super::Renderer2D) has its own sprite texture type,
//! [`GpuTexture`](super::GpuTexture), because a 2D batch caches the bind group
//! it was built with. A 3D model needs the same pixels bound through a
//! different layout, so [`Texture`] keeps only the GPU image and lets each
//! renderer build whatever binding it needs. The two can share one upload with
//! [`Texture::from_gpu_texture`].
//!
//! Pixels are always uploaded as sRGB RGBA8, so sampling returns linear colour
//! and the lit shader needs no gamma conversion.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::graphics::Image;

use super::error::Error;
use super::window::Gpu;

/// Every texture gets an id, used to cache bind groups per renderer.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// An image on the GPU, ready to be sampled by a 3D draw.
///
/// Cloning one is cheap: the wgpu handles are reference-counted, the pixels are
/// not copied. Create one with [`Texture::from_image`], [`Texture::from_rgba8`]
/// or a renderer's `create_texture` convenience methods.
#[derive(Debug, Clone)]
pub struct Texture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
    id: u64,
}

impl Texture {
    /// Uploads an [`Image`], converting it to RGBA8.
    pub fn from_image(gpu: &Gpu, image: &Image) -> Result<Self, Error> {
        let (width, height) = image.size();
        let pixels = image_rgba8(image)?;
        Self::upload(gpu.device(), gpu.queue(), width, height, &pixels)
    }

    /// Uploads tightly packed RGBA8 pixels.
    ///
    /// `pixels` must be exactly `width * height * 4` bytes, one row after
    /// another from top to bottom.
    pub fn from_rgba8(gpu: &Gpu, width: u32, height: u32, pixels: &[u8]) -> Result<Self, Error> {
        Self::upload(gpu.device(), gpu.queue(), width, height, pixels)
    }

    /// Reuses the pixels of a 2D sprite texture.
    ///
    /// This does not copy or re-upload anything: the new handle shares the same
    /// GPU image, so a sprite and a 3D model can show the same picture. The two
    /// handles are independent, and either one keeps the texture alive.
    pub fn from_gpu_texture(texture: &super::GpuTexture) -> Self {
        Texture {
            texture: texture.texture().clone(),
            view: texture.view().clone(),
            size: texture.size(),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Width and height in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.size.0
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.size.1
    }

    /// The underlying texture, for callers building their own bind group.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// The underlying view, for callers building their own bind group.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Identity inside this process, used for bind-group caches.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<Self, Error> {
        check_pixels(width, height, pixels)?;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frglib texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Texture {
            texture,
            view,
            size: (width, height),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        })
    }
}

/// Converts an image to tightly packed RGBA8 bytes.
///
/// SDL pads rows to its own pitch; wgpu wants one row right after another.
pub(crate) fn image_rgba8(image: &Image) -> Result<Vec<u8>, Error> {
    let (width, height) = image.size();
    let converted = image
        .surface
        .convert_format(sdl3::pixels::PixelFormat::RGBA32)?;

    let pitch = converted.pitch() as usize;
    let row_bytes = width as usize * 4;

    Ok(converted.with_lock(|data| {
        if pitch == row_bytes {
            data[..row_bytes * height as usize].to_vec()
        } else {
            let mut packed = Vec::with_capacity(row_bytes * height as usize);
            for row in 0..height as usize {
                let start = row * pitch;
                packed.extend_from_slice(&data[start..start + row_bytes]);
            }
            packed
        }
    }))
}

fn check_pixels(width: u32, height: u32, pixels: &[u8]) -> Result<(), Error> {
    if width == 0 || height == 0 {
        return Err(Error::EmptyTexture);
    }

    let expected = width as usize * height as usize * 4;
    if pixels.len() != expected {
        return Err(Error::PixelData {
            expected,
            actual: pixels.len(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_texture_is_rejected() {
        assert!(matches!(check_pixels(0, 8, &[]), Err(Error::EmptyTexture)));
        assert!(matches!(check_pixels(8, 0, &[]), Err(Error::EmptyTexture)));
    }

    #[test]
    fn the_pixel_count_must_match_the_dimensions() {
        match check_pixels(2, 2, &[0; 15]) {
            Err(Error::PixelData { expected, actual }) => {
                assert_eq!(expected, 16);
                assert_eq!(actual, 15);
            }
            _ => panic!("expected a pixel-data error"),
        }

        assert!(check_pixels(2, 2, &[0; 16]).is_ok());
    }
}
