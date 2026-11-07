use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::graphics::Color;

/// One vertex: position, normal and a colour multiplier.
///
/// The built-in primitives are white, so the colour passed to a draw call
/// decides their shade; a custom mesh can bake per-vertex colours in instead.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex {
    /// A white vertex with `normal` for lighting.
    pub const fn new(position: [f32; 3], normal: [f32; 3]) -> Self {
        Vertex {
            position,
            normal,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// The same vertex with its own colour.
    pub const fn with_color(position: [f32; 3], normal: [f32; 3], color: [f32; 4]) -> Self {
        Vertex {
            position,
            normal,
            color,
        }
    }

    /// How `wgpu` should read [`Vertex`] out of the vertex buffer.
    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: 24,
                shader_location: 2,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

/// CPU-side geometry: vertex and index lists plus how to interpret them.
///
/// Primitive constructors return a mesh in its own local space, centred on the
/// origin. A draw call then places it with a model matrix.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub topology: wgpu::PrimitiveTopology,
}

impl Mesh {
    /// A cube with edge length `size`, centred on the origin.
    pub fn cube(size: f32) -> Self {
        let half = size * 0.5;
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        // Each face has its own four vertices so the normals stay flat. The
        // winding is counter-clockwise seen from outside, matching the
        // `FrontFace::Ccw` / `cull_mode: Back` pipelines.
        quad(
            &mut vertices,
            &mut indices,
            [
                [-half, -half, half],
                [half, -half, half],
                [half, half, half],
                [-half, half, half],
            ],
            [0.0, 0.0, 1.0],
        );
        quad(
            &mut vertices,
            &mut indices,
            [
                [half, -half, -half],
                [-half, -half, -half],
                [-half, half, -half],
                [half, half, -half],
            ],
            [0.0, 0.0, -1.0],
        );
        quad(
            &mut vertices,
            &mut indices,
            [
                [half, -half, half],
                [half, -half, -half],
                [half, half, -half],
                [half, half, half],
            ],
            [1.0, 0.0, 0.0],
        );
        quad(
            &mut vertices,
            &mut indices,
            [
                [-half, -half, -half],
                [-half, -half, half],
                [-half, half, half],
                [-half, half, -half],
            ],
            [-1.0, 0.0, 0.0],
        );
        quad(
            &mut vertices,
            &mut indices,
            [
                [-half, half, half],
                [half, half, half],
                [half, half, -half],
                [-half, half, -half],
            ],
            [0.0, 1.0, 0.0],
        );
        quad(
            &mut vertices,
            &mut indices,
            [
                [-half, -half, -half],
                [half, -half, -half],
                [half, -half, half],
                [-half, -half, half],
            ],
            [0.0, -1.0, 0.0],
        );

        Mesh {
            vertices,
            indices,
            topology: wgpu::PrimitiveTopology::TriangleList,
        }
    }

    /// A flat quad in the XZ plane, facing up.
    pub fn plane(width: f32, depth: f32) -> Self {
        let half_x = width * 0.5;
        let half_z = depth * 0.5;
        let normal = [0.0, 1.0, 0.0];

        Mesh {
            vertices: vec![
                Vertex::new([-half_x, 0.0, half_z], normal),
                Vertex::new([half_x, 0.0, half_z], normal),
                Vertex::new([half_x, 0.0, -half_z], normal),
                Vertex::new([-half_x, 0.0, -half_z], normal),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            topology: wgpu::PrimitiveTopology::TriangleList,
        }
    }

    /// A wireframe grid in the XZ plane.
    ///
    /// `slices` is the number of cells along each axis, `spacing` the width of
    /// one cell in world units.
    pub fn grid(slices: u32, spacing: f32) -> Self {
        let slices = slices.max(1);
        let half = slices as f32 * spacing * 0.5;
        let normal = [0.0, 1.0, 0.0];
        let mut vertices = Vec::with_capacity((slices as usize + 1) * 4);

        for i in 0..=slices {
            let offset = -half + i as f32 * spacing;
            vertices.push(Vertex::new([offset, 0.0, -half], normal));
            vertices.push(Vertex::new([offset, 0.0, half], normal));
            vertices.push(Vertex::new([-half, 0.0, offset], normal));
            vertices.push(Vertex::new([half, 0.0, offset], normal));
        }

        Mesh {
            vertices,
            indices: Vec::new(),
            topology: wgpu::PrimitiveTopology::LineList,
        }
    }

    /// A triangle mesh from raw vertices and indices.
    pub fn from_vertices(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Mesh {
            vertices,
            indices,
            topology: wgpu::PrimitiveTopology::TriangleList,
        }
    }

    /// A list of independent line segments: every pair of vertices is one line.
    pub fn from_line_list(vertices: Vec<Vertex>) -> Self {
        Mesh {
            vertices,
            indices: Vec::new(),
            topology: wgpu::PrimitiveTopology::LineList,
        }
    }

    /// A single line segment, with colours converted to linear space.
    pub fn line(from: Vec3, to: Vec3, color: Color) -> Self {
        let normal = [0.0, 1.0, 0.0];
        let color = super::color_to_linear(color);
        Mesh::from_line_list(vec![
            Vertex::with_color(from.to_array(), normal, color),
            Vertex::with_color(to.to_array(), normal, color),
        ])
    }
}

fn quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
) {
    let base = vertices.len() as u32;
    for corner in corners {
        vertices.push(Vertex::new(corner, normal));
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Geometry uploaded to the GPU.
///
/// Created with [`Renderer3D::create_mesh`](super::Renderer3D::create_mesh) and
/// passed back to a draw call. Cloning one is cheap: it only copies the buffer
/// handles, the GPU memory stays shared.
#[derive(Debug, Clone)]
pub struct GpuMesh {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: Option<wgpu::Buffer>,
    pub(crate) vertex_count: u32,
    pub(crate) index_count: u32,
    pub(crate) topology: wgpu::PrimitiveTopology,
}

impl GpuMesh {
    pub(crate) fn upload(device: &wgpu::Device, mesh: &Mesh) -> Self {
        // `create_buffer_init` rejects an empty slice; an empty mesh is never
        // drawn, but still needs a buffer to make the type hold together.
        const EMPTY: [u8; 4] = [0; 4];
        let vertex_bytes = if mesh.vertices.is_empty() {
            &EMPTY[..]
        } else {
            bytemuck::cast_slice(&mesh.vertices)
        };

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frglib vertices"),
            contents: vertex_bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = (!mesh.indices.is_empty()).then(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("frglib indices"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            })
        });

        GpuMesh {
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.vertices.len() as u32,
            index_count: mesh.indices.len() as u32,
            topology: mesh.topology,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cube_has_six_faces_of_two_triangles() {
        let cube = Mesh::cube(2.0);

        assert_eq!(cube.vertices.len(), 24);
        assert_eq!(cube.indices.len(), 36);
        assert_eq!(cube.topology, wgpu::PrimitiveTopology::TriangleList);
    }

    #[test]
    fn cube_vertices_sit_on_the_requested_edge() {
        let cube = Mesh::cube(4.0);
        let extreme = cube
            .vertices
            .iter()
            .flat_map(|vertex| vertex.position)
            .fold(0.0f32, |acc, value| acc.max(value.abs()));

        assert!((extreme - 2.0).abs() < 1e-6);
    }

    #[test]
    fn every_cube_face_normal_points_away_from_the_centre() {
        let cube = Mesh::cube(1.0);

        for vertex in &cube.vertices {
            let dot = Vec3::from_array(vertex.position).dot(Vec3::from_array(vertex.normal));
            assert!(dot > 0.0, "normal should point outwards, dot was {dot}");
        }
    }

    #[test]
    fn a_plane_is_two_triangles() {
        let plane = Mesh::plane(4.0, 2.0);

        assert_eq!(plane.vertices.len(), 4);
        assert_eq!(plane.indices.len(), 6);
    }

    #[test]
    fn a_grid_is_a_line_list() {
        let grid = Mesh::grid(4, 1.0);

        assert_eq!(grid.topology, wgpu::PrimitiveTopology::LineList);
        assert!(grid.indices.is_empty());
        // Five lines along X and five along Z, two vertices each.
        assert_eq!(grid.vertices.len(), 20);
    }

    #[test]
    fn a_line_has_two_vertices() {
        let line = Mesh::line(Vec3::ZERO, Vec3::X, Color::WHITE);

        assert_eq!(line.topology, wgpu::PrimitiveTopology::LineList);
        assert_eq!(line.vertices.len(), 2);
    }
}
