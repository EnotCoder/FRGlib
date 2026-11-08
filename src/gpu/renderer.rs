use std::collections::HashMap;
use std::mem::size_of;
use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};

use crate::gpu::camera::Camera3D;
use crate::gpu::error::Error;
use crate::gpu::mesh::{GpuMesh, Mesh, Vertex};
use crate::gpu::texture::Texture;
use crate::gpu::window::{Frame, Gpu};
use crate::graphics::{Color, Image};

use super::color_to_linear;

/// How a draw call is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Material {
    /// One directional light plus a little ambient and specular. The default.
    #[default]
    Lit,
    /// The colour goes to the screen unchanged. Good for grids and debug lines.
    Unlit,
    /// A custom material created with [`Renderer3D::create_material`].
    Custom(u32),
}

/// Draws meshes with the built-in camera and light setup.
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

    camera_layout: wgpu::BindGroupLayout,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    draw_layout: wgpu::BindGroupLayout,
    draw_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    draw_capacity: u32,
    draw_stride: u64,

    pipeline_layout: wgpu::PipelineLayout,
    textured_pipeline_layout: wgpu::PipelineLayout,

    lit_triangles: wgpu::RenderPipeline,
    unlit_triangles: wgpu::RenderPipeline,
    unlit_lines: wgpu::RenderPipeline,
    lit_textured: wgpu::RenderPipeline,
    unlit_textured: wgpu::RenderPipeline,

    custom_pipelines: Vec<CustomPipeline>,

    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture_bind_groups: HashMap<u64, wgpu::BindGroup>,

    cube: GpuMesh,
    plane: GpuMesh,
    grid: Option<GridCache>,

    camera: Camera3D,
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
    camera_pos: [f32; 4],
    light_dir: [f32; 4],
    light_color: [f32; 4],
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

        let lit_triangles = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            "fs_lit",
        );
        let unlit_triangles = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            "fs_unlit",
        );
        let unlit_lines = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::LineList,
            None,
            "fs_unlit",
        );
        let lit_textured = create_pipeline(
            &device,
            &textured_pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            "fs_lit_textured",
        );
        let unlit_textured = create_pipeline(
            &device,
            &textured_pipeline_layout,
            &shader,
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
            "fs_unlit_textured",
        );

        let cube = GpuMesh::upload(&device, &Mesh::cube(1.0));
        let plane = GpuMesh::upload(&device, &Mesh::plane(1.0, 1.0));

        Renderer3D {
            device,
            queue,
            format,
            camera_layout,
            camera_buffer,
            camera_bind_group,
            draw_layout,
            draw_buffer,
            draw_bind_group,
            draw_capacity,
            draw_stride,
            pipeline_layout,
            textured_pipeline_layout,
            lit_triangles,
            unlit_triangles,
            unlit_lines,
            lit_textured,
            unlit_textured,
            custom_pipelines: Vec::new(),
            texture_layout,
            sampler,
            texture_bind_groups: HashMap::new(),
            cube,
            plane,
            grid: None,
            camera: Camera3D::perspective(60.0, 0.1, 500.0),
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

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
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
            "fs_main",
        );

        let textured = create_pipeline(
            &self.device,
            &self.textured_pipeline_layout,
            &shader,
            self.format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
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

    /// Starts a frame against `camera`, discarding anything queued before.
    pub fn begin(&mut self, camera: &Camera3D) {
        self.camera = *camera;
        self.commands.clear();
        self.line_vertices.clear();
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
        if mesh.vertex_count == 0 {
            return;
        }

        self.commands.push(DrawCommand {
            mesh: mesh.clone(),
            model: transform,
            color: color_to_linear(color),
            material,
            texture: None,
        });
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
        if mesh.vertex_count == 0 {
            return;
        }

        let bind_group = self.texture_bind_group(texture);

        self.commands.push(DrawCommand {
            mesh: mesh.clone(),
            model: transform,
            color: color_to_linear(color),
            material,
            texture: Some(bind_group),
        });
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
        let camera_uniform = CameraUniform {
            view_proj: self.camera.view_projection(aspect).to_cols_array_2d(),
            camera_pos: self.camera.position.extend(1.0).to_array(),
            light_dir: Vec3::new(-0.4, -1.0, -0.3)
                .normalize()
                .extend(0.0)
                .to_array(),
            light_color: [1.0, 1.0, 1.0, 1.0],
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

        for (index, command) in self.commands.iter().enumerate() {
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
            depth_write_enabled: Some(true),
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
    fn the_camera_uniform_is_four_vec4_after_the_matrix() {
        // mat4x4 (64) + three vec4 (48).
        assert_eq!(size_of::<CameraUniform>(), 112);
    }
}
