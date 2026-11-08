//! A batched 2D renderer: shapes, sprites, interface widgets and text.
//!
//! [`Renderer2D`] is the 2D sibling of [`Renderer3D`](super::Renderer3D). It
//! shares the window, the frame and the swapchain, but writes its own pass:
//! [`begin`](Renderer2D::begin) clears the screen, while
//! [`begin_overlay`](Renderer2D::begin_overlay) draws on top of a 3D scene that
//! has already been recorded.
//!
//! Everything becomes one triangle list. Solid shapes use a 1×1 white texture,
//! text uses a glyph atlas, and sprites use their own texture, so switching
//! between them costs a texture bind rather than a pipeline change.

use std::mem::size_of;
use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};

use crate::graphics::{Camera, Color, Image};
use crate::math::{Rect, Vec2};

use super::error::Error;
use super::font::{FontData, FontId, GlyphAtlas, TextAlign};
use super::window::{Frame, Gpu};
use super::{color_to_linear, DEPTH_FORMAT};

/// A texture that lives on the GPU and can be drawn by [`Renderer2D`].
///
/// Created with [`Renderer2D::create_texture`] or
/// [`Renderer2D::create_texture_rgba8`]. Cloning one is cheap: the GPU memory
/// is shared, only the handle is copied.
///
/// Like a [`FontId`], a texture belongs to the renderer that made it: it holds
/// a bind group built from that renderer's layout and sampler.
#[derive(Debug, Clone)]
pub struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
    /// Unique inside the renderer that made it. Batches compare this instead
    /// of the bind group, which does not implement equality.
    key: u64,
}

impl GpuTexture {
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
}

/// A batched 2D renderer.
///
/// The frame loop mirrors [`Renderer3D`](super::Renderer3D):
///
/// ```no_run
/// # use frglib::{Color, Rect};
/// # use frglib::gpu::{Renderer2D, WgpuWindow};
/// # fn main() -> Result<(), frglib::gpu::Error> {
/// # let mut window = WgpuWindow::builder("demo").build()?;
/// # let mut renderer = Renderer2D::new(window.gpu());
/// # let font = renderer.load_font(std::fs::read("font.otf").unwrap())?;
/// loop {
///     let Some(mut frame) = window.begin_frame()? else { continue };
///     renderer.begin(&frame);
///     renderer.fill_rect(Rect::new(20.0, 20.0, 200.0, 60.0), Color::BLUE);
///     renderer.draw_text(font, "Hello", 28.0, 32.0, 24.0, Color::WHITE)?;
///     renderer.finish(&mut frame);
///     window.present(frame)?;
/// }
/// # }
/// ```
///
/// Draws are queued, not encoded immediately, and [`finish`](Self::finish)
/// records them in one pass. After a 3D scene, use
/// [`begin_overlay`](Self::begin_overlay) so the frame is not cleared again.
pub struct Renderer2D {
    device: wgpu::Device,
    queue: wgpu::Queue,

    pipeline: wgpu::RenderPipeline,

    view_buffer: wgpu::Buffer,
    view_bind_group: wgpu::BindGroup,

    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,

    white: GpuTexture,
    atlas: GlyphAtlas,
    atlas_bind_group: wgpu::BindGroup,
    next_texture_key: u64,

    vertex_buffer: wgpu::Buffer,
    vertex_capacity: u64,
    vertices: Vec<SpriteVertex>,
    batches: Vec<Batch>,

    fonts: Vec<FontData>,

    camera: Camera,
    view_size: (u32, u32),
    screen_space: bool,
    clear: bool,
}

struct Batch {
    key: u64,
    bind_group: wgpu::BindGroup,
    first: u32,
    count: u32,
}

/// Key of the glyph atlas texture.
const ATLAS_KEY: u64 = 0;
/// Key of the 1×1 white texture that solid shapes use.
const WHITE_KEY: u64 = 1;
/// Keys for user textures start here.
const FIRST_TEXTURE_KEY: u64 = 2;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    /// `1.0` for screen pixels, `0.0` for world units.
    space: f32,
    _pad: f32,
}

impl SpriteVertex {
    fn new(position: Vec2, uv: [f32; 2], color: [f32; 4], space: f32) -> Self {
        SpriteVertex {
            position: position.to_array(),
            uv,
            color,
            space,
            _pad: 0.0,
        }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 8,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 16,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32,
                offset: 32,
                shader_location: 3,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<SpriteVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

/// The shader's view uniform: camera, viewport and zoom.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    camera: [f32; 4],
    screen: [f32; 4],
    params: [f32; 4],
}

impl Renderer2D {
    /// Builds the pipeline, the white texture and the glyph atlas.
    pub fn new(gpu: &Gpu) -> Self {
        let device = gpu.device().clone();
        let queue = gpu.queue().clone();
        let format = gpu.format();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frglib 2d shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
        });

        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frglib 2d view layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(size_of::<ViewUniform>() as u64),
                },
                count: None,
            }],
        });

        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frglib 2d view"),
            size: size_of::<ViewUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frglib 2d view"),
            layout: &view_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_buffer.as_entire_binding(),
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frglib 2d texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("frglib 2d sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frglib 2d layout"),
            bind_group_layouts: &[Some(&view_layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frglib 2d pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(SpriteVertex::layout())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // 2D geometry has no meaningful winding: a flipped sprite is
                // still a sprite.
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            // Always pass the depth test and never write: the 2D layer is an
            // overlay, whether it sits on a cleared screen or on a 3D scene.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let white = create_white_texture(&device, &queue, &texture_layout, &sampler);
        let atlas = GlyphAtlas::new(&device);
        let atlas_bind_group =
            create_texture_bind_group(&device, &texture_layout, &sampler, atlas.view());

        let vertex_capacity = 64 * 1024;
        let vertex_buffer = create_vertex_buffer(&device, vertex_capacity);

        Renderer2D {
            device,
            queue,
            pipeline,
            view_buffer,
            view_bind_group,
            texture_layout,
            sampler,
            white,
            atlas,
            atlas_bind_group,
            next_texture_key: FIRST_TEXTURE_KEY,
            vertex_buffer,
            vertex_capacity,
            vertices: Vec::new(),
            batches: Vec::new(),
            fonts: Vec::new(),
            camera: Camera::new(Vec2::ZERO),
            view_size: gpu.size(),
            screen_space: true,
            clear: true,
        }
    }

    /// Parses a TTF/OTF and makes it available to text draws.
    ///
    /// `bytes` are copied into the renderer. Load a file with
    /// `std::fs::read`, or bake it in with `include_bytes!`.
    pub fn load_font(&mut self, bytes: impl Into<Vec<u8>>) -> Result<FontId, Error> {
        let id = FontId(self.fonts.len());
        self.fonts.push(FontData::new(bytes.into())?);
        Ok(id)
    }

    /// Uploads an [`Image`] as a texture.
    ///
    /// The surface is converted to RGBA32 first, so any format SDL can decode
    /// works.
    pub fn create_texture(&mut self, image: &Image) -> Result<GpuTexture, Error> {
        let (width, height) = image.size();
        let converted = image
            .surface
            .convert_format(sdl3::pixels::PixelFormat::RGBA32)?;

        let pitch = converted.pitch() as usize;
        let row_bytes = width as usize * 4;

        // SDL pads rows to its own pitch; wgpu wants tightly packed rows.
        let pixels = converted.with_lock(|data| {
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
        });

        self.create_texture_rgba8(width, height, &pixels)
    }

    /// Uploads tightly packed RGBA8 pixels.
    ///
    /// `pixels` must be exactly `width * height * 4` bytes, one row after
    /// another from top to bottom.
    pub fn create_texture_rgba8(
        &mut self,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<GpuTexture, Error> {
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

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("frglib 2d texture"),
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

        self.queue.write_texture(
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
        let bind_group =
            create_texture_bind_group(&self.device, &self.texture_layout, &self.sampler, &view);

        let key = self.next_texture_key;
        self.next_texture_key += 1;

        Ok(GpuTexture {
            texture,
            view,
            bind_group,
            size: (width, height),
            key,
        })
    }

    /// Starts a frame and clears the colour and depth attachments.
    ///
    /// Use this for a purely 2D frame. After [`Renderer3D::finish`] use
    /// [`begin_overlay`](Self::begin_overlay) instead, or the 3D scene is
    /// cleared away.
    ///
    /// Also drops any draws queued since the last frame.
    ///
    /// [`Renderer3D::finish`]: super::Renderer3D::finish
    pub fn begin(&mut self, frame: &Frame) {
        self.reset(frame.size, true);
    }

    /// Starts a frame on top of whatever is already in the attachments.
    ///
    /// The colour and depth attachments are loaded, not cleared, which is what
    /// makes a 2D overlay on a 3D scene work.
    pub fn begin_overlay(&mut self, frame: &Frame) {
        self.reset(frame.size, false);
    }

    fn reset(&mut self, size: (u32, u32), clear: bool) {
        self.view_size = (size.0.max(1), size.1.max(1));
        // A camera centred on the viewport makes world units equal pixels
        // until the caller sets their own camera.
        self.camera = Camera::new(Vec2::new(
            self.view_size.0 as f32 / 2.0,
            self.view_size.1 as f32 / 2.0,
        ));
        self.screen_space = true;
        self.clear = clear;
        self.vertices.clear();
        self.batches.clear();
    }

    /// The size [`begin`](Self::begin) was given, in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        self.view_size
    }

    /// The camera the world-space draws currently use.
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// Whether positions are currently read as screen pixels.
    pub fn is_screen_space(&self) -> bool {
        self.screen_space
    }

    /// Switches to screen pixels: the default after [`begin`](Self::begin).
    pub fn set_screen_space(&mut self) {
        self.screen_space = true;
    }

    /// Switches to world units and uses `camera` to project them.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
        self.screen_space = false;
    }

    /// Runs `f` in screen space, then restores the previous mode.
    pub fn screen_space<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let was_screen_space = self.screen_space;
        self.screen_space = true;
        let result = f(self);
        self.screen_space = was_screen_space;
        result
    }

    /// Runs `f` through `camera`, then restores the previous camera and mode.
    pub(crate) fn with_camera<R>(&mut self, camera: Camera, f: impl FnOnce(&mut Self) -> R) -> R {
        let previous_camera = self.camera;
        let was_screen_space = self.screen_space;
        self.camera = camera;
        self.screen_space = false;
        let result = f(self);
        self.camera = previous_camera;
        self.screen_space = was_screen_space;
        result
    }

    /// Queues a filled rectangle. `rect.x` and `rect.y` are its top-left corner.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }

        self.push_quad(
            [
                Vec2::new(rect.x, rect.y),
                Vec2::new(rect.x + rect.w, rect.y),
                Vec2::new(rect.x + rect.w, rect.y + rect.h),
                Vec2::new(rect.x, rect.y + rect.h),
            ],
            [0.0, 0.0, 1.0, 1.0],
            color,
            WHITE_KEY,
        );
    }

    /// Queues an outline `thickness` pixels wide just inside `rect`.
    pub fn draw_rect(&mut self, rect: Rect, thickness: f32, color: Color) {
        let thickness = thickness.max(0.0);

        if thickness <= 0.0 || rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }

        // A frame wider than the box has no room for a hole; filling it is the
        // only sensible reading.
        if rect.w <= thickness * 2.0 || rect.h <= thickness * 2.0 {
            self.fill_rect(rect, color);
            return;
        }

        let inner_h = rect.h - thickness * 2.0;
        self.fill_rect(Rect::new(rect.x, rect.y, rect.w, thickness), color);
        self.fill_rect(
            Rect::new(rect.x, rect.y + rect.h - thickness, rect.w, thickness),
            color,
        );
        self.fill_rect(
            Rect::new(rect.x, rect.y + thickness, thickness, inner_h),
            color,
        );
        self.fill_rect(
            Rect::new(
                rect.x + rect.w - thickness,
                rect.y + thickness,
                thickness,
                inner_h,
            ),
            color,
        );
    }

    /// Queues a straight line segment `thickness` pixels wide.
    pub fn draw_line(&mut self, from: Vec2, to: Vec2, thickness: f32, color: Color) {
        let direction = to - from;
        let length = direction.length();

        if length <= f32::EPSILON || thickness <= 0.0 {
            return;
        }

        let normal = Vec2::new(-direction.y, direction.x) / length * (thickness * 0.5);

        self.push_quad(
            [from + normal, to + normal, to - normal, from - normal],
            [0.0, 0.0, 1.0, 1.0],
            color,
            WHITE_KEY,
        );
    }

    /// Queues a filled circle approximated with a triangle fan.
    ///
    /// The segment count grows with the radius, from 12 to 64, which is enough
    /// for an interface and cheap for a particle burst.
    pub fn fill_circle(&mut self, center: Vec2, radius: f32, color: Color) {
        if radius <= 0.0 {
            return;
        }

        let segments = ((radius * 0.5).ceil() as usize).clamp(12, 64);
        let step = std::f32::consts::TAU / segments as f32;

        let mut previous = center + Vec2::new(radius, 0.0);
        let color = color_to_linear(color);
        let space = self.space_flag();

        for index in 1..=segments {
            let angle = step * index as f32;
            let point = center + Vec2::new(angle.cos(), angle.sin()) * radius;
            self.push_triangle(center, previous, point, color, space);
            previous = point;
        }
    }

    /// Queues a whole texture stretched over `dest`, tinted by `tint`.
    pub fn draw_texture(&mut self, texture: &GpuTexture, dest: Rect, tint: Color) {
        let (width, height) = texture.size;
        self.draw_texture_region(
            texture,
            Rect::new(0.0, 0.0, width as f32, height as f32),
            dest,
            tint,
        );
    }

    /// Queues part of a texture, given in source pixels, over `dest`.
    ///
    /// The source rectangle is clamped to the texture, so a sprite sheet that
    /// is a pixel short at the edge does not wrap into its neighbour.
    pub fn draw_texture_region(
        &mut self,
        texture: &GpuTexture,
        source: Rect,
        dest: Rect,
        tint: Color,
    ) {
        if dest.w <= 0.0 || dest.h <= 0.0 {
            return;
        }

        let (width, height) = texture.size;
        let u0 = (source.x / width as f32).clamp(0.0, 1.0);
        let v0 = (source.y / height as f32).clamp(0.0, 1.0);
        let u1 = ((source.x + source.w) / width as f32).clamp(0.0, 1.0);
        let v1 = ((source.y + source.h) / height as f32).clamp(0.0, 1.0);

        self.push_quad_with(
            [
                Vec2::new(dest.x, dest.y),
                Vec2::new(dest.x + dest.w, dest.y),
                Vec2::new(dest.x + dest.w, dest.y + dest.h),
                Vec2::new(dest.x, dest.y + dest.h),
            ],
            [u0, v0, u1, v1],
            tint,
            self.space_flag(),
            texture.key,
            &texture.bind_group,
        );
    }

    /// Queues text with its top-left corner at `(x, y)`.
    ///
    /// `size` is the em size in pixels. Newlines start a new line; there is no
    /// wrapping. Use [`measure_text`](Self::measure_text) to size a box first.
    ///
    /// Text is always drawn in screen pixels, even when the renderer is
    /// currently projecting a world camera: a caption is an interface element,
    /// not a scene object.
    pub fn draw_text(
        &mut self,
        font: FontId,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<(), Error> {
        self.draw_text_aligned(font, text, x, y, size, color, TextAlign::Left)
    }

    /// Queues text anchored to `x` according to `align`, with the top of the
    /// block at `y`.
    pub fn draw_text_aligned(
        &mut self,
        font: FontId,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        align: TextAlign,
    ) -> Result<(), Error> {
        let layout = {
            let data = self.fonts.get(font.0).ok_or(Error::UnknownFont)?;
            data.layout(text, size, [x, y], align)
        };

        for placed in layout.glyphs {
            let glyph = self.fonts[font.0].glyph(&mut self.atlas, &self.queue, placed.ch, size)?;

            if glyph.is_blank() {
                continue;
            }

            let top_left = Vec2::new(
                placed.x + glyph.offset[0],
                placed.baseline + glyph.offset[1],
            );

            let bind_group = self.atlas_bind_group.clone();
            self.push_quad_with(
                [
                    top_left,
                    top_left + Vec2::new(glyph.size[0], 0.0),
                    top_left + Vec2::new(glyph.size[0], glyph.size[1]),
                    top_left + Vec2::new(0.0, glyph.size[1]),
                ],
                glyph.uv,
                color,
                // Text is an interface element: it stays in screen pixels even
                // when the renderer is currently projecting a world camera.
                1.0,
                ATLAS_KEY,
                &bind_group,
            );
        }

        Ok(())
    }

    /// Queues text centred on `(x, y)` both horizontally and vertically.
    pub fn draw_text_centered(
        &mut self,
        font: FontId,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<(), Error> {
        let height = {
            let data = self.fonts.get(font.0).ok_or(Error::UnknownFont)?;
            data.layout(text, size, [0.0, 0.0], TextAlign::Center)
                .height
        };

        self.draw_text_aligned(
            font,
            text,
            x,
            y - height / 2.0,
            size,
            color,
            TextAlign::Center,
        )
    }

    /// Measures a text block in pixels: the widest line and the height of all
    /// lines together.
    pub fn measure_text(&self, font: FontId, text: &str, size: f32) -> Result<Vec2, Error> {
        let data = self.fonts.get(font.0).ok_or(Error::UnknownFont)?;
        let layout = data.layout(text, size, [0.0, 0.0], TextAlign::Left);
        Ok(Vec2::new(layout.width, layout.height))
    }

    /// Records the queued draws into `frame` and clears the queue.
    ///
    /// The pass is always written: [`begin`](Self::begin) is expected to clear
    /// the frame even when nothing was drawn.
    pub fn finish(&mut self, frame: &mut Frame) {
        // An overlay with no draws has nothing to add; skipping the pass keeps
        // an empty HUD free.
        if self.vertices.is_empty() && !self.clear {
            self.batches.clear();
            return;
        }

        if !self.vertices.is_empty() {
            let byte_len = (self.vertices.len() * size_of::<SpriteVertex>()) as u64;
            self.ensure_vertex_capacity(byte_len);
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }

        let zoom = if self.camera.zoom > f32::EPSILON {
            self.camera.zoom
        } else {
            1.0
        };

        let view = ViewUniform {
            camera: [self.camera.position.x, self.camera.position.y, 0.0, 1.0],
            screen: [self.view_size.0 as f32, self.view_size.1 as f32, 0.0, 0.0],
            params: [zoom, 0.0, 0.0, 0.0],
        };

        self.queue
            .write_buffer(&self.view_buffer, 0, bytemuck::bytes_of(&view));

        let Frame {
            encoder,
            color,
            depth,
            background,
            ..
        } = frame;

        let (color_load, depth_load) = if self.clear {
            (
                wgpu::LoadOp::Clear(super::color_to_wgpu(*background)),
                wgpu::LoadOp::Clear(1.0),
            )
        } else {
            (wgpu::LoadOp::Load, wgpu::LoadOp::Load)
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frglib 2d"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &*color,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: color_load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &*depth,
                depth_ops: Some(wgpu::Operations {
                    load: depth_load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        for batch in &self.batches {
            pass.set_bind_group(1, &batch.bind_group, &[]);
            pass.draw(batch.first..batch.first + batch.count, 0..1);
        }

        drop(pass);

        self.vertices.clear();
        self.batches.clear();
    }

    fn space_flag(&self) -> f32 {
        if self.screen_space {
            1.0
        } else {
            0.0
        }
    }

    fn push_quad(&mut self, corners: [Vec2; 4], uv: [f32; 4], color: Color, key: u64) {
        // The white texture is the only one whose bind group is owned here.
        let bind_group = self.white.bind_group.clone();
        let space = self.space_flag();
        self.push_quad_with(corners, uv, color, space, key, &bind_group);
    }

    fn push_quad_with(
        &mut self,
        corners: [Vec2; 4],
        uv: [f32; 4],
        color: Color,
        space: f32,
        key: u64,
        bind_group: &wgpu::BindGroup,
    ) {
        let color = color_to_linear(color);
        let [u0, v0, u1, v1] = uv;

        let vertices = [
            SpriteVertex::new(corners[0], [u0, v0], color, space),
            SpriteVertex::new(corners[1], [u1, v0], color, space),
            SpriteVertex::new(corners[2], [u1, v1], color, space),
            SpriteVertex::new(corners[0], [u0, v0], color, space),
            SpriteVertex::new(corners[2], [u1, v1], color, space),
            SpriteVertex::new(corners[3], [u0, v1], color, space),
        ];

        let first = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&vertices);
        self.push_batch(key, bind_group, first, 6);
    }

    fn push_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: [f32; 4], space: f32) {
        let first = self.vertices.len() as u32;
        self.vertices
            .push(SpriteVertex::new(a, [0.0, 0.0], color, space));
        self.vertices
            .push(SpriteVertex::new(b, [0.0, 0.0], color, space));
        self.vertices
            .push(SpriteVertex::new(c, [0.0, 0.0], color, space));

        let bind_group = self.white.bind_group.clone();
        self.push_batch(WHITE_KEY, &bind_group, first, 3);
    }

    fn push_batch(&mut self, key: u64, bind_group: &wgpu::BindGroup, first: u32, count: u32) {
        if let Some(batch) = self.batches.last_mut() {
            if batch.key == key {
                batch.count += count;
                return;
            }
        }

        self.batches.push(Batch {
            key,
            bind_group: bind_group.clone(),
            first,
            count,
        });
    }

    fn ensure_vertex_capacity(&mut self, needed: u64) {
        if needed <= self.vertex_capacity {
            return;
        }

        let capacity = needed.max(self.vertex_capacity.saturating_mul(2));
        self.vertex_buffer = create_vertex_buffer(&self.device, capacity);
        self.vertex_capacity = capacity;
    }
}

fn create_vertex_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("frglib 2d vertices"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("frglib 2d texture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn create_white_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
) -> GpuTexture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frglib white"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
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
        &[255, 255, 255, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = create_texture_bind_group(device, layout, sampler, &view);

    GpuTexture {
        texture,
        view,
        bind_group,
        size: (1, 1),
        key: WHITE_KEY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vertex_is_the_size_the_layout_describes() {
        // vec2 (8) + vec2 (8) + vec4 (16) + f32 (4) + padding (4).
        assert_eq!(size_of::<SpriteVertex>(), 40);
    }

    #[test]
    fn the_view_uniform_is_three_vec4() {
        assert_eq!(size_of::<ViewUniform>(), 48);
    }

    #[test]
    fn the_vertex_layout_covers_the_useful_bytes() {
        let layout = SpriteVertex::layout();
        let offsets: Vec<u64> = layout
            .attributes
            .iter()
            .map(|attribute| attribute.offset)
            .collect();

        assert_eq!(layout.array_stride, 40);
        // vec2, vec2, vec4, then the space flag; the last four bytes are the
        // padding that keeps the struct 4-byte aligned.
        assert_eq!(offsets, vec![0, 8, 16, 32]);
    }
}
