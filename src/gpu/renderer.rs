use std::collections::HashMap;
use std::mem::size_of;
use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};

use crate::gpu::camera::Camera3D;
use crate::gpu::error::Error;
use crate::gpu::gltf::{GltfModel, GpuGltfModel, GpuGltfPrimitive};
use crate::gpu::light::{Light, LightKind, MAX_LIGHTS};
use crate::gpu::mesh::{GpuMesh, Mesh, Vertex};
use crate::gpu::skybox::Skybox;
use crate::gpu::texture::Texture;
use crate::gpu::window::{Frame, Gpu};
use crate::graphics::{Color, Image};

use super::color_to_linear;

/// How a draw call is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Material {
    /// Every active light in the scene plus a little ambient and specular.
    /// The default.
    #[default]
    Lit,
    /// The colour goes to the screen unchanged. Good for grids and debug lines.
    Unlit,
    /// Unlit colour blended with alpha, without writing depth.
    ///
    /// The renderer draws these after every opaque draw and sorts them from
    /// far to near, so a glow, a billboard or a fading particle cannot hide a
    /// model behind it. Use it for anything whose colour carries transparency.
    Transparent,
    /// A custom material created with [`Renderer3D::create_material`].
    Custom(u32),
}

impl Material {
    /// Whether this material is alpha-blended and drawn after opaque geometry.
    pub fn is_transparent(self) -> bool {
        matches!(self, Material::Transparent)
    }
}

/// Draws meshes with the built-in camera, light and sky setup.
///
/// Call [`begin`](Self::begin), queue some draws, then [`finish`](Self::finish)
/// to record them into the frame:
///
/// ```no_run
/// # use frglib::{Color, Event};
/// # use frglib::gpu::{Camera3D, Renderer3D, WgpuWindow};
/// # use frglib::gpu::glam::Vec3;
/// # fn main() -> Result<(), frglib::gpu::Error> {
/// # let mut window = WgpuWindow::builder("demo").build()?;
/// # let mut renderer = Renderer3D::new(window.gpu());
/// # let camera = Camera3D::perspective(60.0, 0.1, 100.0);
/// # let Some(mut frame) = window.begin_frame()? else { return Ok(()) };
/// renderer.begin(&camera);
/// renderer.draw_cube(Vec3::ZERO, Vec3::ONE, Color::RED);
/// renderer.draw_grid(20, 1.0, Color::WHITE);
/// renderer.finish(&mut frame);
/// # window.present(frame)?;
/// # Ok(())
/// # }
/// ```
///
/// Draws are queued, not encoded immediately: this keeps the render pass in one
/// place and makes it possible to batch the per-draw uniforms. The queue is
/// cleared by both [`begin`](Self::begin) and [`finish`](Self::finish).
///
/// The pass this records clears the colour and depth attachments with the
/// frame's background. Raw `wgpu` passes recorded into the same [`Frame`] should
/// come after it and use `LoadOp::Load`.
pub struct Renderer3D {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    draw_layout: wgpu::BindGroupLayout,
    draw_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    draw_capacity: u32,
    draw_stride: u64,

    pipeline_layout: wgpu::PipelineLayout,
    textured_pipeline_layout: wgpu::PipelineLayout,

    skybox_layout: wgpu::BindGroupLayout,
    skybox_pipeline: wgpu::RenderPipeline,
    skybox_sampler: wgpu::Sampler,
    skybox: Option<Skybox>,
    skybox_bind_group: Option<wgpu::BindGroup>,

    lit_triangles: wgpu::RenderPipeline,
    unlit_triangles: wgpu::RenderPipeline,
    unlit_lines: wgpu::RenderPipeline,
    lit_textured: wgpu::RenderPipeline,
    unlit_textured: wgpu::RenderPipeline,
    transparent_triangles: wgpu::RenderPipeline,
    transparent_lines: wgpu::RenderPipeline,
    transparent_textured: wgpu::RenderPipeline,

    custom_pipelines: Vec<CustomPipeline>,

    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture_bind_groups: HashMap<u64, wgpu::BindGroup>,

    cube: GpuMesh,
    plane: GpuMesh,
    grid: Option<GridCache>,

    camera: Camera3D,
    lights: Vec<Light>,
    ambient: Color,
    commands: Vec<DrawCommand>,
    line_vertices: Vec<Vertex>,
}

struct CustomPipeline {
    triangles: wgpu::RenderPipeline,
    textured: wgpu::RenderPipeline,
}

struct GridCache {
    slices: u32,
    spacing_bits: u32,
    mesh: GpuMesh,
}

struct DrawCommand {
    mesh: GpuMesh,
    model: Mat4,
    color: [f32; 4],
    material: Material,
    /// Resolved texture binding, when the draw samples a texture.
    texture: Option<wgpu::BindGroup>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    /// `rgb` is linear colour, `a` is intensity.
    ambient: [f32; 4],
    /// `x` is the number of active lights; the rest pads the vec4.
    light_count: [u32; 4],
    /// `xyz` is the direction (directional) or position (point); `w` is 0 or 1.
    light_position: [[f32; 4]; MAX_LIGHTS],
    /// `rgb` is linear colour, `a` is intensity.
    light_color: [[f32; 4]; MAX_LIGHTS],
    /// `x` is the range of a point light.
    light_params: [[f32; 4]; MAX_LIGHTS],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DrawUniform {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

impl Renderer3D {
    /// Builds the pipelines and the built-in meshes.
    ///
    /// The built-in shader is compiled synchronously; a broken shader is a bug
    /// in this crate and shows up as a `wgpu` validation error, not as an
    /// `Err` here.
    pub fn new(gpu: &Gpu) -> Self {
        let device = gpu.device().clone();
        let queue = gpu.queue().clone();
        let format = gpu.format();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frglib 3d shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/lit.wgsl").into()),
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frglib camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(size_of::<CameraUniform>() as u64),
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frglib camera"),
            size: size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frglib camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frglib draw layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(size_of::<DrawUniform>() as u64),
                },
                count: None,
            }],
        });

        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let draw_stride = align_up(size_of::<DrawUniform>() as u64, alignment);
        let draw_capacity = 256;
        let (draw_buffer, draw_bind_group) = create_draw_bindings(
            &device,
            &draw_layout,
            draw_stride,
            draw_capacity,
            size_of::<DrawUniform>() as u64,
        );

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frglib 3d texture layout"),
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
            label: Some("frglib 3d sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frglib 3d layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&draw_layout)],
            immediate_size: 0,
        });

        let textured_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("frglib 3d textured layout"),
                bind_group_layouts: &[
                    Some(&camera_layout),
                    Some(&draw_layout),
                    Some(&texture_layout),
                ],
                immediate_size: 0,
            });

        let skybox_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frglib skybox layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
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

        let skybox_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("frglib skybox sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        // The skybox binds group 0 (camera) and group 3 (cube map). Groups 1
        // and 2 are the draw and albedo groups, which the skybox does not use.
        let skybox_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("frglib skybox pipeline layout"),
                bind_group_layouts: &[Some(&camera_layout), None, None, Some(&skybox_layout)],
                immediate_size: 0,
            });

        let skybox_pipeline =
            create_skybox_pipeline(&device, &skybox_pipeline_layout, &shader, format);

        let lit_triangles = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            true,
            "fs_lit",
        );
        let unlit_triangles = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            true,
            "fs_unlit",
        );
        let unlit_lines = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::LineList,
            None,
            true,
            "fs_unlit",
        );
        let lit_textured = create_pipeline(
            &device,
            &textured_pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            true,
            "fs_lit_textured",
        );
        let unlit_textured = create_pipeline(
            &device,
            &textured_pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            true,
            "fs_unlit_textured",
        );

        // Alpha-blended draws share the unlit shader, but skip depth writes so
        // a glow cannot punch a hole in whatever stands behind it.
        let transparent_triangles = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            false,
            "fs_unlit",
        );
        let transparent_lines = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::LineList,
            None,
            false,
            "fs_unlit",
        );
        let transparent_textured = create_pipeline(
            &device,
            &textured_pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            false,
            "fs_unlit_textured",
        );

        let cube = GpuMesh::upload(&device, &Mesh::cube(1.0));
        let plane = GpuMesh::upload(&device, &Mesh::plane(1.0, 1.0));

        Renderer3D {
            device,
            queue,
            format,
            camera_buffer,
            camera_bind_group,
            draw_layout,
            draw_buffer,
            draw_bind_group,
            draw_capacity,
            draw_stride,
            pipeline_layout,
            textured_pipeline_layout,
            skybox_layout,
            skybox_pipeline,
            skybox_sampler,
            skybox: None,
            skybox_bind_group: None,
            lit_triangles,
            unlit_triangles,
            unlit_lines,
            lit_textured,
            unlit_textured,
            transparent_triangles,
            transparent_lines,
            transparent_textured,
            custom_pipelines: Vec::new(),
            texture_layout,
            sampler,
            texture_bind_groups: HashMap::new(),
            cube,
            plane,
            grid: None,
            camera: Camera3D::perspective(60.0, 0.1, 500.0),
            // Keep the historical single directional light until the caller
            // sets their own. `clear_lights` starts an empty scene.
            lights: vec![Light::directional(
                Vec3::new(-0.4, -1.0, -0.3),
                Color::WHITE,
            )],
            ambient: Color::rgb(46, 46, 46), // ~0.18 in sRGB
            commands: Vec::new(),
            line_vertices: Vec::new(),
        }
    }

    /// Creates a custom material from a WGSL shader.
    ///
    /// The shader must provide the same entry points as the built-in one:
    /// `vs_main`, `fs_main` and `fs_main_textured`. It can use the same
    /// bind groups (0 for camera, 1 for draw, 2 for texture).
    ///
    /// The returned `Material` can be passed to `draw_mesh_with`.
    pub fn create_material(&mut self, source: &str) -> Material {
        let full_source = format!(
            "{}\n{}",
            include_str!("shaders/lit.wgsl")
                .split("@vertex")
                .next()
                .unwrap(),
            source
        );

        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("frglib custom shader"),
                source: wgpu::ShaderSource::Wgsl(full_source.into()),
            });

        let triangles = create_pipeline(
            &self.device,
            &self.pipeline_layout,
            &shader,
            self.format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            true,
            "fs_main",
        );

        let textured = create_pipeline(
            &self.device,
            &self.textured_pipeline_layout,
            &shader,
            self.format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            true,
            "fs_main_textured",
        );

        let id = self.custom_pipelines.len() as u32;
        self.custom_pipelines.push(CustomPipeline {
            triangles,
            textured,
        });

        Material::Custom(id)
    }

    /// Uploads caller geometry to the GPU.
    ///
    /// The returned handle can be cloned and kept across frames; the buffers
    /// are reference-counted, not copied.
    pub fn create_mesh(&self, mesh: &Mesh) -> GpuMesh {
        GpuMesh::upload(&self.device, mesh)
    }

    /// Uploads an [`Image`] as a texture this renderer can sample.
    ///
    /// The result can be passed to [`draw_mesh_textured`](Self::draw_mesh_textured)
    /// any number of times and across frames. For a model loaded from OBJ, this
    /// is the picture its `vt` coordinates point into.
    pub fn create_texture(&self, image: &Image) -> Result<Texture, Error> {
        let (width, height) = image.size();
        let pixels = super::texture::image_rgba8(image)?;
        self.create_texture_rgba8(width, height, &pixels)
    }

    /// Uploads tightly packed RGBA8 pixels as a texture.
    ///
    /// `pixels` must be exactly `width * height * 4` bytes, one row after
    /// another from top to bottom.
    pub fn create_texture_rgba8(
        &self,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<Texture, Error> {
        Texture::upload(&self.device, &self.queue, width, height, pixels)
    }

    /// Builds a skybox from an equirectangular panorama.
    ///
    /// This is the same as [`Skybox::from_equirectangular`], using the device
    /// this renderer already holds. The result can be passed to
    /// [`set_skybox`](Self::set_skybox).
    pub fn create_skybox(&self, image: &Image) -> Result<Skybox, Error> {
        let (width, height) = image.size();
        let pixels = super::texture::image_rgba8(image)?;
        Skybox::from_equirectangular_pixels(&self.device, &self.queue, width, height, &pixels)
    }

    /// Uploads a whole [`GltfModel`] to the GPU.
    ///
    /// Every unique primitive becomes a mesh, every decoded image a texture.
    /// The returned handle can be cloned and drawn across frames.
    ///
    /// ```no_run
    /// use frglib::gpu::{GltfModel, Renderer3D};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let renderer: Renderer3D = todo!();
    /// let model = GltfModel::from_file("models/robot.glb")?;
    /// let gpu_model = renderer.create_gltf(&model)?;
    /// # let _ = gpu_model;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_gltf(&self, model: &GltfModel) -> Result<GpuGltfModel, Error> {
        let primitives = model
            .primitives
            .iter()
            .map(|primitive| GpuGltfPrimitive {
                mesh: self.create_mesh(&primitive.mesh),
                material: primitive.material,
            })
            .collect();

        let textures = model
            .images
            .iter()
            .map(|image| self.create_texture_rgba8(image.width, image.height, &image.rgba8))
            .collect::<Result<_, _>>()?;

        Ok(GpuGltfModel {
            primitives,
            instances: model.instances.clone(),
            materials: model.materials.clone(),
            textures,
        })
    }

    /// Starts a frame against `camera`, discarding anything queued before.
    pub fn begin(&mut self, camera: &Camera3D) {
        self.camera = *camera;
        self.commands.clear();
        self.line_vertices.clear();
    }

    /// Replaces every light in the scene.
    ///
    /// At most [`MAX_LIGHTS`] are kept; extra entries are dropped. An empty
    /// iterator leaves only the ambient term, so a dark scene is possible on
    /// purpose. The built-in default is one white directional light.
    pub fn set_lights(&mut self, lights: impl IntoIterator<Item = Light>) {
        self.lights = lights.into_iter().take(MAX_LIGHTS).collect();
    }

    /// Adds one light to the scene.
    ///
    /// Lights persist across frames until [`clear_lights`](Self::clear_lights)
    /// or [`set_lights`](Self::set_lights) replaces them. Once [`MAX_LIGHTS`]
    /// are present, further lights are ignored.
    pub fn add_light(&mut self, light: Light) {
        if self.lights.len() < MAX_LIGHTS {
            self.lights.push(light);
        }
    }

    /// Removes every light, leaving only the ambient term.
    pub fn clear_lights(&mut self) {
        self.lights.clear();
    }

    /// The lights currently used by lit draws.
    pub fn lights(&self) -> &[Light] {
        &self.lights
    }

    /// Sets the global ambient light colour.
    ///
    /// The default is a dark grey (~0.18 linear), which keeps the shadowed
    /// sides of objects from being pitch black.
    pub fn set_ambient(&mut self, color: Color) {
        self.ambient = color;
    }

    /// The current global ambient light colour.
    pub fn ambient(&self) -> Color {
        self.ambient
    }

    /// Sets the background cube map drawn behind the scene.
    ///
    /// The skybox is drawn first in every [`finish`](Self::finish), so it fills
    /// whatever the frame's clear leaves and the scene is drawn on top. It
    /// ignores the world position and only follows the camera's rotation.
    pub fn set_skybox(&mut self, skybox: Skybox) {
        self.skybox_bind_group = Some(create_skybox_bind_group(
            &self.device,
            &self.skybox_layout,
            &self.skybox_sampler,
            skybox.view(),
        ));
        self.skybox = Some(skybox);
    }

    /// Removes the skybox, leaving the frame's clear colour.
    pub fn clear_skybox(&mut self) {
        self.skybox = None;
        self.skybox_bind_group = None;
    }

    /// The skybox currently drawn, if any.
    pub fn skybox(&self) -> Option<&Skybox> {
        self.skybox.as_ref()
    }

    /// Queues a lit mesh draw.
    pub fn draw_mesh(&mut self, mesh: &GpuMesh, transform: Mat4, color: Color) {
        self.draw_mesh_with(mesh, transform, Material::Lit, color);
    }

    /// Queues a mesh draw with an explicit [`Material`].
    pub fn draw_mesh_with(
        &mut self,
        mesh: &GpuMesh,
        transform: Mat4,
        material: Material,
        color: Color,
    ) {
        self.queue_mesh(mesh, transform, color_to_linear(color), material, None);
    }

    /// Queues a lit mesh draw that samples `texture`.
    ///
    /// The vertex colours and the `color` argument multiply the sampled texel,
    /// so pass [`Color::WHITE`] to show the image unchanged. Texture
    /// coordinates come from the mesh: OBJ models carry them, and the built-in
    /// cube and plane map the image onto every face.
    pub fn draw_mesh_textured(
        &mut self,
        mesh: &GpuMesh,
        texture: &Texture,
        transform: Mat4,
        color: Color,
    ) {
        self.draw_mesh_textured_with(mesh, texture, transform, Material::Lit, color);
    }

    /// Queues a textured mesh draw with an explicit [`Material`].
    pub fn draw_mesh_textured_with(
        &mut self,
        mesh: &GpuMesh,
        texture: &Texture,
        transform: Mat4,
        material: Material,
        color: Color,
    ) {
        let bind_group = self.texture_bind_group(texture);
        self.queue_mesh(
            mesh,
            transform,
            color_to_linear(color),
            material,
            Some(bind_group),
        );
    }

    /// Queues every instance of a [`GpuGltfModel`].
    ///
    /// `transform` is applied on top of each node's own world transform, so it
    /// can place, rotate or scale the whole scene. Materials contribute their
    /// base-colour factor and texture; `KHR_materials_unlit` materials use the
    /// unlit pipeline.
    ///
    /// ```no_run
    /// use frglib::gpu::glam::Mat4;
    /// # use frglib::gpu::{GpuGltfModel, Renderer3D};
    /// # fn draw(renderer: &mut Renderer3D, model: &GpuGltfModel) {
    /// renderer.draw_gltf(model, Mat4::IDENTITY);
    /// # }
    /// ```
    pub fn draw_gltf(&mut self, model: &GpuGltfModel, transform: Mat4) {
        for instance in &model.instances {
            let Some(primitive) = model.primitives.get(instance.primitive) else {
                continue;
            };

            let placement = transform * instance.transform;

            let (color, material, texture) = match primitive
                .material
                .and_then(|index| model.materials.get(index))
            {
                Some(material) => (
                    material.base_color,
                    if material.unlit {
                        Material::Unlit
                    } else {
                        Material::Lit
                    },
                    material
                        .base_color_texture
                        .and_then(|index| model.textures.get(index)),
                ),
                None => ([1.0; 4], Material::Lit, None),
            };

            match texture {
                Some(texture) => {
                    let bind_group = self.texture_bind_group(texture);
                    self.queue_mesh(
                        &primitive.mesh,
                        placement,
                        color,
                        material,
                        Some(bind_group),
                    );
                }
                None => self.queue_mesh(&primitive.mesh, placement, color, material, None),
            }
        }
    }

    /// Queues a textured unit cube moved and scaled into place.
    pub fn draw_cube_textured(
        &mut self,
        center: Vec3,
        size: Vec3,
        texture: &Texture,
        color: Color,
    ) {
        let model = Mat4::from_translation(center) * Mat4::from_scale(size);
        let cube = self.cube.clone();
        self.draw_mesh_textured(&cube, texture, model, color);
    }

    /// Queues a textured flat XZ quad, scaled by `size`.
    pub fn draw_plane_textured(
        &mut self,
        center: Vec3,
        size: Vec2,
        texture: &Texture,
        color: Color,
    ) {
        let model =
            Mat4::from_translation(center) * Mat4::from_scale(Vec3::new(size.x, 1.0, size.y));
        let plane = self.plane.clone();
        self.draw_mesh_textured(&plane, texture, model, color);
    }

    /// Queues a unit cube moved and scaled into place.
    pub fn draw_cube(&mut self, center: Vec3, size: Vec3, color: Color) {
        let model = Mat4::from_translation(center) * Mat4::from_scale(size);
        self.draw_cube_model(model, color);
    }

    /// Queues the unit cube with an arbitrary model matrix, for rotation.
    pub fn draw_cube_model(&mut self, model: Mat4, color: Color) {
        let cube = self.cube.clone();
        self.draw_mesh(&cube, model, color);
    }

    /// Queues a flat XZ quad, scaled by `size`.
    pub fn draw_plane(&mut self, center: Vec3, size: Vec2, color: Color) {
        let model =
            Mat4::from_translation(center) * Mat4::from_scale(Vec3::new(size.x, 1.0, size.y));
        let plane = self.plane.clone();
        self.draw_mesh(&plane, model, color);
    }

    /// Queues a wireframe grid in the XZ plane.
    ///
    /// The mesh is cached; asking for the same `slices` and `spacing` again
    /// reuses the buffers.
    pub fn draw_grid(&mut self, slices: u32, spacing: f32, color: Color) {
        let grid = self.grid_mesh(slices, spacing);
        self.commands.push(DrawCommand {
            mesh: grid,
            model: Mat4::IDENTITY,
            color: color_to_linear(color),
            material: Material::Unlit,
            texture: None,
        });
    }

    /// Queues one line segment.
    ///
    /// Lines issued this way are batched into a single buffer per frame, so a
    /// few hundred debug lines do not create a buffer each.
    pub fn draw_line(&mut self, from: Vec3, to: Vec3, color: Color) {
        let normal = [0.0, 1.0, 0.0];
        let color = color_to_linear(color);
        self.line_vertices
            .push(Vertex::with_color(from.to_array(), normal, color));
        self.line_vertices
            .push(Vertex::with_color(to.to_array(), normal, color));
    }

    /// Queues a connected polyline through `points`.
    pub fn draw_lines(&mut self, points: &[Vec3], color: Color) {
        for pair in points.windows(2) {
            self.draw_line(pair[0], pair[1], color);
        }
    }

    /// Records the queued draws into `frame` and clears the queue.
    pub fn finish(&mut self, frame: &mut Frame) {
        if !self.line_vertices.is_empty() {
            let mesh = Mesh::from_line_list(std::mem::take(&mut self.line_vertices));
            let gpu = GpuMesh::upload(&self.device, &mesh);
            self.commands.push(DrawCommand {
                mesh: gpu,
                model: Mat4::IDENTITY,
                color: [1.0; 4],
                material: Material::Unlit,
                texture: None,
            });
        }

        self.ensure_draw_capacity(self.commands.len() as u32);

        let Frame {
            encoder,
            color,
            depth,
            background,
            size,
            ..
        } = frame;

        let aspect = *size;
        let aspect = aspect.0 as f32 / aspect.1.max(1) as f32;

        let mut light_position = [[0.0f32; 4]; MAX_LIGHTS];
        let mut light_color = [[0.0f32; 4]; MAX_LIGHTS];
        let mut light_params = [[0.0f32; 4]; MAX_LIGHTS];

        for (index, light) in self.lights.iter().take(MAX_LIGHTS).enumerate() {
            match light.kind {
                LightKind::Directional { direction } => {
                    let direction = direction.normalize_or_zero();
                    // A zero vector would turn into NaN in the shader; aim a
                    // broken light straight down instead.
                    let direction = if direction == Vec3::ZERO {
                        Vec3::NEG_Y
                    } else {
                        direction
                    };
                    light_position[index] = direction.extend(0.0).to_array();
                }
                LightKind::Point { position, range } => {
                    light_position[index] = position.extend(1.0).to_array();
                    light_params[index][0] = range.max(0.0);
                }
            }

            let color = color_to_linear(light.color);
            light_color[index] = [color[0], color[1], color[2], light.intensity.max(0.0)];
        }

        let view_proj = self.camera.view_projection(aspect);

        let camera_uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            inv_view_proj: view_proj.inverse().to_cols_array_2d(),
            camera_pos: self.camera.position.extend(1.0).to_array(),
            ambient: color_to_linear(self.ambient),
            light_count: [self.lights.len().min(MAX_LIGHTS) as u32, 0, 0, 0],
            light_position,
            light_color,
            light_params,
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));

        for (index, command) in self.commands.iter().enumerate() {
            let uniform = DrawUniform {
                model: command.model.to_cols_array_2d(),
                color: command.color,
            };
            self.queue.write_buffer(
                &self.draw_buffer,
                index as u64 * self.draw_stride,
                bytemuck::bytes_of(&uniform),
            );
        }

        // Opaque geometry first, then alpha-blended draws from far to near.
        // The order is separate from the command indices because the uniform
        // buffer is indexed by the original queue position.
        let order = self.draw_order();

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frglib 3d"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &*color,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(super::color_to_wgpu(*background)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &*depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_bind_group(0, &self.camera_bind_group, &[]);

        if let Some(skybox) = &self.skybox_bind_group {
            pass.set_pipeline(&self.skybox_pipeline);
            pass.set_bind_group(3, skybox, &[]);
            pass.draw(0..3, 0..1);
        }

        for index in order {
            let command = &self.commands[index];
            // A texture only makes sense on triangles; lines have no sampler
            // and their pipeline layout has no texture group.
            let textured = command.texture.is_some()
                && command.mesh.topology == wgpu::PrimitiveTopology::TriangleList;

            let pipeline = match (command.material, command.mesh.topology, textured) {
                (Material::Lit, wgpu::PrimitiveTopology::TriangleList, true) => &self.lit_textured,
                (Material::Lit, wgpu::PrimitiveTopology::TriangleList, false) => {
                    &self.lit_triangles
                }
                (Material::Unlit, wgpu::PrimitiveTopology::TriangleList, true) => {
                    &self.unlit_textured
                }
                (Material::Unlit, wgpu::PrimitiveTopology::TriangleList, false) => {
                    &self.unlit_triangles
                }
                (Material::Transparent, wgpu::PrimitiveTopology::TriangleList, true) => {
                    &self.transparent_textured
                }
                (Material::Transparent, wgpu::PrimitiveTopology::TriangleList, false) => {
                    &self.transparent_triangles
                }
                (Material::Transparent, wgpu::PrimitiveTopology::LineList, _) => {
                    &self.transparent_lines
                }
                (Material::Custom(id), wgpu::PrimitiveTopology::TriangleList, true) => {
                    &self.custom_pipelines[id as usize].textured
                }
                (Material::Custom(id), wgpu::PrimitiveTopology::TriangleList, false) => {
                    &self.custom_pipelines[id as usize].triangles
                }
                (_, wgpu::PrimitiveTopology::LineList, _) => &self.unlit_lines,
                _ => &self.unlit_triangles,
            };

            pass.set_pipeline(pipeline);
            pass.set_bind_group(
                1,
                &self.draw_bind_group,
                &[(index as u64 * self.draw_stride) as u32],
            );

            if textured {
                if let Some(texture) = &command.texture {
                    pass.set_bind_group(2, texture, &[]);
                }
            }

            pass.set_vertex_buffer(0, command.mesh.vertex_buffer.slice(..));

            match &command.mesh.index_buffer {
                Some(indices) => {
                    pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..command.mesh.index_count, 0, 0..1);
                }
                None => pass.draw(0..command.mesh.vertex_count, 0..1),
            }
        }

        drop(pass);
        self.commands.clear();
    }

    /// Indices of the queued draws in render order.
    ///
    /// Opaque draws keep their queue order; transparent ones go last and are
    /// sorted by distance from the camera, farthest first.
    fn draw_order(&self) -> Vec<usize> {
        let camera = self.camera.position;
        let mut order: Vec<usize> = (0..self.commands.len()).collect();

        order.sort_by(|&a, &b| {
            let first = &self.commands[a];
            let second = &self.commands[b];
            let first_transparent = first.material.is_transparent();
            let second_transparent = second.material.is_transparent();

            first_transparent
                .cmp(&second_transparent)
                .then_with(|| {
                    if first_transparent {
                        let first_distance =
                            first.model.w_axis.truncate().distance_squared(camera);
                        let second_distance =
                            second.model.w_axis.truncate().distance_squared(camera);

                        // Far to near: the farther draw comes first.
                        second_distance
                            .partial_cmp(&first_distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
        });

        order
    }

    fn queue_mesh(
        &mut self,
        mesh: &GpuMesh,
        model: Mat4,
        color: [f32; 4],
        material: Material,
        texture: Option<wgpu::BindGroup>,
    ) {
        if mesh.vertex_count == 0 {
            return;
        }

        self.commands.push(DrawCommand {
            mesh: mesh.clone(),
            model,
            color,
            material,
            texture,
        });
    }

    fn texture_bind_group(&mut self, texture: &Texture) -> wgpu::BindGroup {
        if let Some(group) = self.texture_bind_groups.get(&texture.id()) {
            return group.clone();
        }

        let group = create_texture_bind_group(
            &self.device,
            &self.texture_layout,
            &self.sampler,
            texture.view(),
        );
        self.texture_bind_groups.insert(texture.id(), group.clone());
        group
    }

    fn grid_mesh(&mut self, slices: u32, spacing: f32) -> GpuMesh {
        let spacing_bits = spacing.to_bits();

        if let Some(cache) = &self.grid {
            if cache.slices == slices && cache.spacing_bits == spacing_bits {
                return cache.mesh.clone();
            }
        }

        let mesh = GpuMesh::upload(&self.device, &Mesh::grid(slices, spacing));
        self.grid = Some(GridCache {
            slices,
            spacing_bits,
            mesh: mesh.clone(),
        });
        mesh
    }

    fn ensure_draw_capacity(&mut self, needed: u32) {
        if needed <= self.draw_capacity {
            return;
        }

        // Grow geometrically so a scene that slowly gains instances does not
        // rebuild the buffer every frame.
        let capacity = needed.max(self.draw_capacity.saturating_mul(2));
        let (buffer, bind_group) = create_draw_bindings(
            &self.device,
            &self.draw_layout,
            self.draw_stride,
            capacity,
            size_of::<DrawUniform>() as u64,
        );

        self.draw_buffer = buffer;
        self.draw_bind_group = bind_group;
        self.draw_capacity = capacity;
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    topology: wgpu::PrimitiveTopology,
    cull: Option<wgpu::Face>,
    depth_write: bool,
    fragment_entry: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("frglib 3d pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(Vertex::layout())],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: cull,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: super::DEPTH_FORMAT,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(wgpu::CompareFunction::Less),
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
    })
}

fn create_skybox_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("frglib skybox pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_skybox"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_skybox"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: super::DEPTH_FORMAT,
            // The sky sits on the far plane and must not hide the scene.
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
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
    })
}

fn create_skybox_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("frglib skybox"),
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

fn create_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("frglib 3d texture"),
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

fn create_draw_bindings(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    stride: u64,
    capacity: u32,
    binding_size: u64,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("frglib draw uniforms"),
        size: stride * capacity as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("frglib draw uniforms"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: NonZeroU64::new(binding_size),
            }),
        }],
    });

    (buffer, bind_group)
}

fn align_up(value: u64, alignment: u64) -> u64 {
    let alignment = alignment.max(1);
    value.div_ceil(alignment) * alignment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniforms_are_aligned_to_the_device_stride() {
        assert_eq!(align_up(80, 256), 256);
        assert_eq!(align_up(256, 256), 256);
        assert_eq!(align_up(257, 256), 512);
    }

    #[test]
    fn a_zero_alignment_still_moves_forward() {
        assert_eq!(align_up(80, 0), 80);
    }

    #[test]
    fn the_draw_uniform_is_the_size_the_shader_expects() {
        // mat4x4 (64) + vec4 (16).
        assert_eq!(size_of::<DrawUniform>(), 80);
    }

    #[test]
    fn the_camera_uniform_has_room_for_every_light_and_the_skybox() {
        // view_proj (64) + inv_view_proj (64) + camera vec4 (16) + ambient
        // vec4 (16) + count vec4 (16) + three light arrays of MAX_LIGHTS vec4s.
        assert_eq!(size_of::<CameraUniform>(), 176 + MAX_LIGHTS * 3 * 16);
    }
}
