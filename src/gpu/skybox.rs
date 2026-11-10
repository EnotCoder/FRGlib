//! Cubemap skies for the built-in 3D renderer.
//!
//! A [`Skybox`] is a cube texture sampled by direction. It can be built either
//! from six face images or from one equirectangular panorama, which is
//! converted to six faces on the CPU. [`Renderer3D`](super::Renderer3D) draws
//! it behind everything else with [`Renderer3D::set_skybox`](super::Renderer3D::set_skybox).

use glam::Vec3;

use crate::graphics::Image;

use super::error::Error;
use super::texture::{check_pixels, image_rgba8};
use super::window::Gpu;

/// The order of cube faces: `+X`, `-X`, `+Y`, `-Y`, `+Z`, `-Z`.
pub const SKYBOX_FACES: usize = 6;

/// A cubemap used as the background of a 3D scene.
///
/// The texture is uploaded as sRGB RGBA8. Cloning is cheap: the wgpu handles
/// are reference-counted.
#[derive(Debug, Clone)]
pub struct Skybox {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    face_size: u32,
}

impl Skybox {
    /// Builds a skybox from an equirectangular panorama.
    ///
    /// The image is treated as a 2:1 spherical map: `u = 0.5` looks along `+Z`,
    /// `u = 0` along `-Z`, the top edge at `+Y` and the bottom edge at `-Y`.
    /// It is converted to six cube faces with a size of a quarter of the
    /// panorama width (clamped to 16..=1024 pixels).
    pub fn from_equirectangular(gpu: &Gpu, image: &Image) -> Result<Self, Error> {
        let (width, height) = image.size();
        let pixels = image_rgba8(image)?;
        Self::from_equirectangular_pixels(gpu.device(), gpu.queue(), width, height, &pixels)
    }

    /// Builds a skybox from tightly packed RGBA8 equirectangular pixels.
    ///
    /// `pixels` must be exactly `width * height * 4` bytes, one row after
    /// another from top to bottom.
    pub fn from_equirectangular_rgba8(
        gpu: &Gpu,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<Self, Error> {
        Self::from_equirectangular_pixels(gpu.device(), gpu.queue(), width, height, pixels)
    }

    /// Builds a skybox from six square face images.
    ///
    /// The order is `+X`, `-X`, `+Y`, `-Y`, `+Z`, `-Z`. All faces must have
    /// the same size.
    pub fn from_faces(gpu: &Gpu, faces: [&Image; SKYBOX_FACES]) -> Result<Self, Error> {
        let mut rgba = Vec::with_capacity(SKYBOX_FACES);
        for face in faces {
            rgba.push(image_rgba8(face)?);
        }

        let (width, height) = faces[0].size();
        for (index, face) in faces.iter().enumerate() {
            let size = face.size();
            if size != (width, height) {
                return Err(Error::SkyboxFaceSize {
                    face: index,
                    expected: (width, height),
                    actual: size,
                });
            }
        }

        Self::from_rgba8_faces(
            gpu,
            width,
            height,
            [&rgba[0], &rgba[1], &rgba[2], &rgba[3], &rgba[4], &rgba[5]],
        )
    }

    /// Builds a skybox from six tightly packed RGBA8 faces.
    ///
    /// The order is `+X`, `-X`, `+Y`, `-Y`, `+Z`, `-Z`. Every face must be
    /// square and exactly `width * height * 4` bytes.
    pub fn from_rgba8_faces(
        gpu: &Gpu,
        width: u32,
        height: u32,
        faces: [&[u8]; SKYBOX_FACES],
    ) -> Result<Self, Error> {
        Self::upload(gpu.device(), gpu.queue(), width, height, faces)
    }

    /// The width and height of one cube face, in pixels.
    pub fn face_size(&self) -> u32 {
        self.face_size
    }

    /// The underlying texture, for callers building their own bind group.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// The cube view, for callers building their own bind group.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub(crate) fn from_equirectangular_pixels(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<Self, Error> {
        check_pixels(width, height, pixels)?;

        let face_size = (width / 4).clamp(16, 1024);
        let mut faces: [Vec<u8>; SKYBOX_FACES] =
            std::array::from_fn(|_| Vec::with_capacity((face_size * face_size * 4) as usize));

        for (face, out) in faces.iter_mut().enumerate() {
            for y in 0..face_size {
                let t = (y as f32 + 0.5) / face_size as f32;
                for x in 0..face_size {
                    let s = (x as f32 + 0.5) / face_size as f32;
                    let direction = face_direction(face, s, t);
                    let (u, v) = equirect_uv(direction);
                    let texel = sample_equirect(pixels, width, height, u, v);
                    out.extend_from_slice(&texel);
                }
            }
        }

        Self::upload(
            device,
            queue,
            face_size,
            face_size,
            [
                &faces[0], &faces[1], &faces[2], &faces[3], &faces[4], &faces[5],
            ],
        )
    }

    fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        faces: [&[u8]; SKYBOX_FACES],
    ) -> Result<Self, Error> {
        if width == 0 || height == 0 {
            return Err(Error::EmptyTexture);
        }
        if width != height {
            return Err(Error::SkyboxNotSquare { width, height });
        }

        for face in faces {
            check_pixels(width, height, face)?;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frglib skybox"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: SKYBOX_FACES as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (index, face) in faces.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: index as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                face,
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
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("frglib skybox"),
            format: None,
            dimension: Some(wgpu::TextureViewDimension::Cube),
            usage: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: Some(SKYBOX_FACES as u32),
        });

        Ok(Self {
            texture,
            view,
            face_size: width,
        })
    }
}

/// A direction pointing out of the centre of one cube face.
///
/// `s` and `t` run from `0` to `1`; the mapping matches the cube-map sampling
/// convention, including `t = 0` at the top of the face.
fn face_direction(face: usize, s: f32, t: f32) -> Vec3 {
    let sc = s * 2.0 - 1.0;
    let tc = t * 2.0 - 1.0;

    match face {
        0 => Vec3::new(1.0, -tc, -sc),
        1 => Vec3::new(-1.0, -tc, sc),
        2 => Vec3::new(sc, 1.0, tc),
        3 => Vec3::new(sc, -1.0, -tc),
        4 => Vec3::new(sc, -tc, 1.0),
        _ => Vec3::new(-sc, -tc, -1.0),
    }
}

/// The `(u, v)` of an equirectangular panorama that looks along `direction`.
fn equirect_uv(direction: Vec3) -> (f32, f32) {
    let direction = direction.normalize_or_zero();
    let u = 0.5 + direction.x.atan2(direction.z) / std::f32::consts::TAU;
    let v = 0.5 - direction.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI;
    (u.rem_euclid(1.0), v.clamp(0.0, 1.0))
}

/// Bilinear sample with horizontal wrap and vertical clamp.
fn sample_equirect(pixels: &[u8], width: u32, height: u32, u: f32, v: f32) -> [u8; 4] {
    let w = width as f32;
    let h = height as f32;

    let x = u.rem_euclid(1.0) * w - 0.5;
    let y = (v.clamp(0.0, 1.0) * h - 0.5).clamp(0.0, h - 1.0);

    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;

    let x0 = (x0 as i64).rem_euclid(width as i64) as u32;
    let x1 = (x0 + 1) % width;
    let y0 = y0.clamp(0.0, (height - 1) as f32) as u32;
    let y1 = (y0 + 1).min(height - 1);

    let texel = |x: u32, y: u32| -> [f32; 4] {
        let index = ((y * width + x) * 4) as usize;
        [
            pixels[index] as f32,
            pixels[index + 1] as f32,
            pixels[index + 2] as f32,
            pixels[index + 3] as f32,
        ]
    };

    let top = mix(texel(x0, y0), texel(x1, y0), fx);
    let bottom = mix(texel(x0, y1), texel(x1, y1), fx);
    let color = mix(top, bottom, fy);

    [
        color[0].round().clamp(0.0, 255.0) as u8,
        color[1].round().clamp(0.0, 255.0) as u8,
        color[2].round().clamp(0.0, 255.0) as u8,
        color[3].round().clamp(0.0, 255.0) as u8,
    ]
}

fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_top_face_points_up_and_the_bottom_points_down() {
        assert!((face_direction(2, 0.5, 0.5) - Vec3::Y).length() < 1e-6);
        assert!((face_direction(3, 0.5, 0.5) - Vec3::NEG_Y).length() < 1e-6);
    }

    #[test]
    fn the_side_faces_point_along_their_axes() {
        assert!((face_direction(0, 0.5, 0.5) - Vec3::X).length() < 1e-6);
        assert!((face_direction(1, 0.5, 0.5) - Vec3::NEG_X).length() < 1e-6);
        assert!((face_direction(4, 0.5, 0.5) - Vec3::Z).length() < 1e-6);
        assert!((face_direction(5, 0.5, 0.5) - Vec3::NEG_Z).length() < 1e-6);
    }

    #[test]
    fn the_equirectangular_map_places_the_poles_at_the_edges() {
        let (_, top) = equirect_uv(Vec3::Y);
        let (_, bottom) = equirect_uv(Vec3::NEG_Y);
        let (front, middle) = equirect_uv(Vec3::Z);

        assert!(top.abs() < 1e-6);
        assert!((bottom - 1.0).abs() < 1e-6);
        assert!((middle - 0.5).abs() < 1e-6);
        assert!((front - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_face_from_a_panorama_keeps_the_sky_on_top() {
        // 4x2 panorama: the top row is red, the bottom row is blue.
        let pixels = [
            255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255, 0, 0,
            255, 255, 0, 0, 255, 255, 0, 0, 255, 255,
        ];

        let top = sample_equirect(&pixels, 4, 2, 0.5, 0.0);
        let bottom = sample_equirect(&pixels, 4, 2, 0.5, 1.0);

        assert_eq!(top, [255, 0, 0, 255]);
        assert_eq!(bottom, [0, 0, 255, 255]);
    }
}
