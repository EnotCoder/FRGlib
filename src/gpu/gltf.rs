//! glTF 2.0 loading.
//!
//! The loader reads a whole scene, not just one mesh: every mesh primitive keeps
//! its material, and every node instance keeps its world transform. The result
//! is a [`GltfModel`] on the CPU, ready to be uploaded with
//! [`Renderer3D::create_gltf`](super::Renderer3D::create_gltf) and drawn with
//! [`Renderer3D::draw_gltf`](super::Renderer3D::draw_gltf).
//!
//! Both `.gltf` (JSON, external or embedded buffers) and `.glb` (binary) files
//! work through [`GltfModel::from_file`]; [`GltfModel::from_slice`] takes a GLB
//! or JSON blob that is already in memory.
//!
//! Only the pieces the renderer can show are read: positions, normals, UVs,
//! vertex colours, skin weights, triangle/line topology, the base-colour
//! factor and its texture, the `KHR_materials_unlit` and `KHR_texture_transform`
//! extensions. Skins and transformation animations are loaded too; see
//! [`GltfAnimationPlayer`](super::GltfAnimationPlayer). Morph targets, cameras,
//! lights, texture samplers and the other PBR channels are still ignored.
//! The texture's `TEXCOORD_n` set is baked into the mesh, so a material that
//! asks for set 1 samples set 1. A mesh without normals is shaded flat, and a
//! mesh without UVs maps every vertex to `(0, 0)`.
//!
//! Unlike OBJ, glTF already counts UVs from the top-left corner, which is the
//! convention [`Texture`](super::Texture) uses, so the V axis is not flipped.

use std::fmt;
use std::io;
use std::path::Path;

use glam::{Mat4, Quat, Vec2, Vec3};

use super::mesh::{GpuMesh, Mesh, Vertex};
use super::texture::Texture;

/// What went wrong while reading a glTF model.
#[derive(Debug)]
pub enum GltfError {
    /// The file, its buffers or one of its images could not be read.
    Import(gltf::Error),
    /// The file parsed, but contains no geometry the renderer can draw.
    Empty,
    /// A skin has more joints than the shared shader buffer can hold.
    TooManyJoints { skin: usize, count: usize },
}

/// Joint matrices reserved per skin in the shared GPU buffer.
///
/// This must stay in sync with `MAX_JOINTS` in `shaders/lit.wgsl`.
pub const MAX_JOINTS: usize = 64;

impl fmt::Display for GltfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GltfError::Import(err) => write!(f, "could not read the glTF file: {err}"),
            GltfError::Empty => write!(f, "the glTF file has no drawable geometry"),
            GltfError::TooManyJoints { skin, count } => write!(
                f,
                "glTF skin {skin} has {count} joints, but only {MAX_JOINTS} are supported",
            ),
        }
    }
}

impl std::error::Error for GltfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GltfError::Import(err) => Some(err),
            GltfError::Empty | GltfError::TooManyJoints { .. } => None,
        }
    }
}

impl From<gltf::Error> for GltfError {
    fn from(err: gltf::Error) -> Self {
        GltfError::Import(err)
    }
}

impl From<io::Error> for GltfError {
    fn from(err: io::Error) -> Self {
        GltfError::Import(gltf::Error::Io(err))
    }
}

/// One primitive of a glTF mesh: geometry plus the material it uses.
#[derive(Debug, Clone)]
pub struct GltfPrimitive {
    /// CPU-side geometry, already triangulated or converted to a line list.
    pub mesh: Mesh,
    /// Index into [`GltfModel::materials`], when the primitive names one.
    pub material: Option<usize>,
}

/// One node that draws a primitive.
///
/// The loader flattens the scene graph: an instance already stores the world
/// transform of its node, so a whole model is just a list of instances. Node and
/// skin indices are kept so an animated draw can replace that static transform
/// with the current pose.
#[derive(Debug, Clone)]
pub struct GltfInstance {
    /// Index into [`GltfModel::primitives`].
    pub primitive: usize,
    /// Index into [`GltfModel::nodes`] for the node that owns the primitive.
    pub node: usize,
    /// Index into [`GltfModel::skins`], when the node is skinned.
    pub skin: Option<usize>,
    /// Rest-pose world transform of the node that owns the primitive.
    pub transform: Mat4,
}

/// One node of a glTF scene graph.
///
/// The loader keeps the local TRS values instead of just a matrix, because
/// animation channels write translation, rotation and scale independently.
#[derive(Debug, Clone)]
pub struct GltfNode {
    /// Local translation, in the parent's space.
    pub translation: Vec3,
    /// Local rotation quaternion.
    pub rotation: Quat,
    /// Local scale.
    pub scale: Vec3,
    /// Parent node index, or `None` for a root.
    pub parent: Option<usize>,
    /// Child node indices.
    pub children: Vec<usize>,
    /// Skin used by a mesh attached to this node.
    pub skin: Option<usize>,
}

impl Default for GltfNode {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            parent: None,
            children: Vec::new(),
            skin: None,
        }
    }
}

impl GltfNode {
    /// Composes the node's local transform: T * R * S, like glTF does.
    pub fn local_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale)
    }
}

/// A glTF skin: joint nodes and their inverse bind matrices.
#[derive(Debug, Clone)]
pub struct GltfSkin {
    /// Node indices of the joints, in the order used by `JOINTS_0`.
    pub joints: Vec<usize>,
    /// Inverse of each joint's global transform in the bind pose.
    pub inverse_bind_matrices: Vec<Mat4>,
    /// Node used as the skeleton root, when the file names one.
    pub skeleton_root: Option<usize>,
}

/// Which local transform an animation channel writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfAnimationProperty {
    /// Translation (XYZ).
    Translation,
    /// Rotation quaternion (XYZW).
    Rotation,
    /// Scale (XYZ).
    Scale,
}

/// How an animation channel interpolates between two keyframes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GltfInterpolation {
    /// Linear interpolation, with slerp for rotations.
    Linear,
    /// Hold the previous keyframe until the next one.
    Step,
    /// Cubic spline tangents are ignored: the middle value of each triple is
    /// interpolated like [`Linear`](Self::Linear).
    CubicSpline,
}

/// One animation channel: a timed series of values for one node property.
#[derive(Debug, Clone)]
pub struct GltfAnimationChannel {
    /// Node the channel animates.
    pub node: usize,
    /// Property the channel writes.
    pub property: GltfAnimationProperty,
    /// Interpolation between samples.
    pub interpolation: GltfInterpolation,
    /// `(time, value)` pairs. Rotation values are XYZW, the others XYZ.
    pub samples: Vec<(f32, [f32; 4])>,
}

/// One glTF animation clip: a set of channels sharing a timeline.
#[derive(Debug, Clone)]
pub struct GltfAnimation {
    /// Channels of the clip.
    pub channels: Vec<GltfAnimationChannel>,
    /// Time of the last keyframe, in seconds.
    pub duration: f32,
}

/// The base colour of a glTF material.
#[derive(Debug, Clone)]
pub struct GltfMaterial {
    /// Linear RGBA base-colour factor. glTF stores this value in linear space,
    /// so it is passed to the shader without an sRGB conversion.
    pub base_color: [f32; 4],
    /// Index into [`GltfModel::images`] for the base-colour texture.
    pub base_color_texture: Option<usize>,
    /// Which `TEXCOORD_n` set the base-colour texture samples.
    pub base_color_tex_coord: u32,
    /// `true` when the material asks for `KHR_materials_unlit`.
    pub unlit: bool,
}

/// The `KHR_texture_transform` parameters of one texture reference.
#[derive(Debug, Clone, Copy)]
struct UvTransform {
    offset: Vec2,
    rotation: f32,
    scale: Vec2,
}

impl UvTransform {
    fn apply(self, uv: Vec2) -> Vec2 {
        // The extension rotates around the UV origin, after scaling and before
        // the translation: `uv' = rotate(uv * scale) + offset`.
        let scaled = uv * self.scale;
        let (sin, cos) = self.rotation.sin_cos();
        Vec2::new(
            scaled.x * cos - scaled.y * sin,
            scaled.x * sin + scaled.y * cos,
        ) + self.offset
    }
}

/// A decoded image from the glTF file, ready to be uploaded as a texture.
#[derive(Debug, Clone)]
pub struct GltfImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Tightly packed RGBA8 pixels, top row first.
    pub rgba8: Vec<u8>,
}

/// A glTF scene on the CPU.
///
/// `primitives` holds each unique mesh primitive once, while `instances` says
/// where and how many times it is drawn. That keeps a model that reuses one
/// mesh in several nodes from being duplicated in memory or on the GPU.
#[derive(Debug, Clone)]
pub struct GltfModel {
    /// Unique mesh primitives.
    pub primitives: Vec<GltfPrimitive>,
    /// Node instances, with world transforms already applied.
    pub instances: Vec<GltfInstance>,
    /// Scene graph nodes, indexed the same way as in the glTF document.
    pub nodes: Vec<GltfNode>,
    /// Skins referenced by the nodes.
    pub skins: Vec<GltfSkin>,
    /// Animation clips stored in the file.
    pub animations: Vec<GltfAnimation>,
    /// Materials referenced by the primitives.
    pub materials: Vec<GltfMaterial>,
    /// Decoded base-colour images used by the materials.
    pub images: Vec<GltfImage>,
}

/// Composes each node's local matrix with its parents'.
///
/// `locals` may hold animated transforms instead of the rest pose; glTF
/// hierarchies are not required to list parents before their children.
pub(crate) fn node_world_matrices(nodes: &[GltfNode], locals: &[Mat4]) -> Vec<Mat4> {
    fn visit(
        index: usize,
        nodes: &[GltfNode],
        locals: &[Mat4],
        world: &mut [Mat4],
        state: &mut [u8],
    ) {
        if state[index] != 0 {
            return;
        }
        state[index] = 1;
        // A malformed cyclic hierarchy is treated as a root rather than
        // recursing forever.
        let parent = nodes[index]
            .parent
            .filter(|parent| *parent < nodes.len() && state[*parent] != 1);
        if let Some(parent) = parent {
            visit(parent, nodes, locals, world, state);
        }
        world[index] = parent.map_or(locals[index], |parent| world[parent] * locals[index]);
        state[index] = 2;
    }

    let mut world = vec![Mat4::IDENTITY; nodes.len()];
    let mut state = vec![0u8; nodes.len()];
    for index in 0..nodes.len() {
        visit(index, nodes, locals, &mut world, &mut state);
    }
    world
}

/// Turns a set of world-space joint transforms into skinning matrices.
pub(crate) fn joint_matrices(skin: &GltfSkin, world: &[Mat4]) -> Vec<Mat4> {
    skin.joints
        .iter()
        .zip(&skin.inverse_bind_matrices)
        .map(|(joint, inverse_bind)| {
            world.get(*joint).copied().unwrap_or(Mat4::IDENTITY) * *inverse_bind
        })
        .collect()
}

/// One glTF primitive uploaded to the GPU.
#[derive(Debug, Clone)]
pub struct GpuGltfPrimitive {
    /// Geometry in vertex and index buffers.
    pub mesh: GpuMesh,
    /// Index into [`GpuGltfModel::materials`].
    pub material: Option<usize>,
}

/// A glTF scene uploaded to the GPU.
///
/// Create one with [`Renderer3D::create_gltf`](super::Renderer3D::create_gltf)
/// and pass it to [`Renderer3D::draw_gltf`](super::Renderer3D::draw_gltf).
/// Cloning it is cheap: the buffers and textures are reference-counted.
#[derive(Debug, Clone)]
pub struct GpuGltfModel {
    /// Uploaded mesh primitives, in the same order as [`GltfModel::primitives`].
    pub primitives: Vec<GpuGltfPrimitive>,
    /// Node instances, in the same order as [`GltfModel::instances`].
    pub instances: Vec<GltfInstance>,
    /// Material values, in the same order as [`GltfModel::materials`].
    pub materials: Vec<GltfMaterial>,
    /// Uploaded base-colour textures, in the same order as [`GltfModel::images`].
    pub textures: Vec<Texture>,
    /// Bind-pose joint matrices for each skin, used by the plain draw calls.
    pub bind_pose: Vec<Vec<Mat4>>,
}

impl GltfModel {
    /// Reads a `.gltf` or `.glb` file and all of its buffers and images.
    ///
    /// External files are resolved relative to the model, so a `.gltf` that
    /// points at `crate.bin` and `crate.png` next to it just works.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, GltfError> {
        let (document, buffers, images) = gltf::import(path)?;
        Self::from_parts(document, &buffers, images)
    }

    /// Reads a glTF or GLB document from a byte slice.
    ///
    /// Embedded images work for both forms: a GLB stores them in its binary
    /// chunk, while a JSON document may use `data:` URIs. External buffers and
    /// images are resolved relative to the current working directory, because
    /// a slice has no path of its own.
    pub fn from_slice(bytes: impl AsRef<[u8]>) -> Result<Self, GltfError> {
        let bytes = bytes.as_ref();
        let gltf = gltf::Gltf::from_slice(bytes)?;
        let base = Path::new(".");
        let buffers = gltf::import_buffers(&gltf.document, Some(base), gltf.blob)?;
        let images = gltf::import_images(&gltf.document, Some(base), &buffers)?;
        Self::from_parts(gltf.document, &buffers, images)
    }

    fn from_parts(
        document: gltf::Document,
        buffers: &[gltf::buffer::Data],
        images: Vec<gltf::image::Data>,
    ) -> Result<Self, GltfError> {
        let images: Vec<GltfImage> = images.iter().map(image_to_rgba8).collect();

        let mut materials = Vec::new();
        let mut uv_transforms: Vec<Option<UvTransform>> = Vec::new();

        for material in document.materials() {
            let pbr = material.pbr_metallic_roughness();
            let info = pbr.base_color_texture();

            let mut tex_coord = info.as_ref().map_or(0, |info| info.tex_coord());
            let mut transform = None;

            if let Some(info) = &info {
                if let Some(extension) = info.texture_transform() {
                    if let Some(override_set) = extension.tex_coord() {
                        tex_coord = override_set;
                    }
                    transform = Some(UvTransform {
                        offset: Vec2::from_array(extension.offset()),
                        rotation: extension.rotation(),
                        scale: Vec2::from_array(extension.scale()),
                    });
                }
            }

            materials.push(GltfMaterial {
                base_color: pbr.base_color_factor(),
                base_color_texture: info.as_ref().map(|info| info.texture().source().index()),
                base_color_tex_coord: tex_coord,
                unlit: material.unlit(),
            });
            uv_transforms.push(transform);
        }

        // Every glTF primitive becomes one entry; `(mesh, primitive)` remembers
        // which entry belongs to which document primitive so nodes can refer
        // back to it.
        let mut primitives = Vec::new();
        let mut primitive_indices = Vec::new();

        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let material = primitive.material().index();
                // Pick the UV set the base-colour texture actually samples.
                // The renderer has one UV channel per vertex, so the choice is
                // baked into the mesh when the primitive is loaded.
                let (tex_coord, transform) = material
                    .and_then(|index| {
                        Some((
                            materials.get(index)?.base_color_tex_coord,
                            uv_transforms.get(index).copied().flatten(),
                        ))
                    })
                    .unwrap_or((0, None));

                if let Some(geometry) = build_mesh(&primitive, buffers, tex_coord, transform) {
                    let index = primitives.len();
                    primitives.push(GltfPrimitive {
                        mesh: geometry,
                        material,
                    });
                    primitive_indices.push((mesh.index(), primitive.index(), index));
                }
            }
        }

        let mut instances = Vec::new();
        let mut node_slots: Vec<Option<GltfNode>> = vec![None; document.nodes().count()];
        let scene = document
            .default_scene()
            .or_else(|| document.scenes().next());

        match scene {
            Some(scene) => {
                for node in scene.nodes() {
                    visit_node(
                        &node,
                        None,
                        Mat4::IDENTITY,
                        &primitive_indices,
                        &mut node_slots,
                        &mut instances,
                    );
                }
            }
            // A document without a scene is unusual but still usable: treat
            // every node as a root.
            None => {
                for node in document.nodes() {
                    visit_node(
                        &node,
                        None,
                        Mat4::IDENTITY,
                        &primitive_indices,
                        &mut node_slots,
                        &mut instances,
                    );
                }
            }
        }
        let nodes: Vec<GltfNode> = node_slots
            .into_iter()
            .map(|node| node.unwrap_or_default())
            .collect();

        let mut skins = Vec::new();
        for skin in document.skins() {
            let reader = skin.reader(|buffer| buffers.get(buffer.index()).map(|data| &data.0[..]));
            let inverse_bind_matrices = reader
                .read_inverse_bind_matrices()
                .map(|matrices| {
                    matrices
                        .map(|matrix| Mat4::from_cols_array_2d(&matrix))
                        .collect()
                })
                .unwrap_or_default();
            skins.push(GltfSkin {
                joints: skin.joints().map(|joint| joint.index()).collect(),
                inverse_bind_matrices,
                skeleton_root: skin.skeleton().map(|node| node.index()),
            });
        }
        if let Some((index, skin)) = skins
            .iter()
            .enumerate()
            .find(|(_, skin)| skin.joints.len() > MAX_JOINTS)
        {
            return Err(GltfError::TooManyJoints {
                skin: index,
                count: skin.joints.len(),
            });
        }

        let animations = document
            .animations()
            .map(|animation| read_animation(&animation, buffers))
            .collect();

        if primitives.is_empty() || instances.is_empty() {
            return Err(GltfError::Empty);
        }

        Ok(GltfModel {
            primitives,
            instances,
            nodes,
            skins,
            animations,
            materials,
            images,
        })
    }

    /// Joint matrices for a skin in the model's rest pose.
    ///
    /// Pass these to a draw when no animation is playing. Deforming a vertex
    /// with them leaves it where the modelling tool placed it.
    pub fn bind_pose(&self, skin: usize) -> Option<Vec<Mat4>> {
        let skin = self.skins.get(skin)?;
        let locals: Vec<Mat4> = self.nodes.iter().map(GltfNode::local_matrix).collect();
        Some(joint_matrices(
            skin,
            &node_world_matrices(&self.nodes, &locals),
        ))
    }
}

fn visit_node(
    node: &gltf::Node<'_>,
    parent: Option<usize>,
    parent_world: Mat4,
    primitive_indices: &[(usize, usize, usize)],
    nodes: &mut [Option<GltfNode>],
    instances: &mut Vec<GltfInstance>,
) {
    let index = node.index();
    let transform = node.transform();
    let skin = node.skin().map(|skin| skin.index());
    let (translation, rotation, scale) = transform.clone().decomposed();
    nodes[index] = Some(GltfNode {
        translation: Vec3::from_array(translation),
        rotation: Quat::from_array(rotation),
        scale: Vec3::from_array(scale),
        parent,
        children: node.children().map(|child| child.index()).collect(),
        skin,
    });

    let local = Mat4::from_cols_array_2d(&transform.matrix());
    let world = parent_world * local;

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            if let Some((_, _, primitive_index)) =
                primitive_indices.iter().find(|(mesh_index, index, _)| {
                    *mesh_index == mesh.index() && *index == primitive.index()
                })
            {
                instances.push(GltfInstance {
                    primitive: *primitive_index,
                    node: index,
                    skin,
                    transform: world,
                });
            }
        }
    }

    for child in node.children() {
        visit_node(
            &child,
            Some(index),
            world,
            primitive_indices,
            nodes,
            instances,
        );
    }
}

/// Reads one primitive and converts its topology into a [`Mesh`].
///
/// Returns `None` for point primitives, which the renderer has no pipeline for,
/// and for primitives without positions.
fn build_mesh(
    primitive: &gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
    tex_coord: u32,
    transform: Option<UvTransform>,
) -> Option<Mesh> {
    use gltf::mesh::Mode;

    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| &data.0[..]));
    let mode = primitive.mode();
    let positions: Vec<[f32; 3]> = reader.read_positions()?.collect();
    if positions.is_empty() {
        return None;
    }

    let normals: Option<Vec<[f32; 3]>> = reader.read_normals().map(|iter| iter.collect());
    // A material may ask for a UV set the mesh does not have; falling back to
    // set 0 keeps the texture visible instead of collapsing every UV to zero.
    let uvs: Option<Vec<[f32; 2]>> = reader
        .read_tex_coords(tex_coord)
        .or_else(|| {
            if tex_coord == 0 {
                None
            } else {
                reader.read_tex_coords(0)
            }
        })
        .map(read_tex_coords)
        .map(|uvs| match transform {
            Some(transform) => uvs
                .into_iter()
                .map(|uv| transform.apply(Vec2::from_array(uv)).to_array())
                .collect(),
            None => uvs,
        });
    let colors: Option<Vec<[f32; 4]>> = reader.read_colors(0).map(read_colors);
    // Joint influences are optional. Empty lists mean an ordinary, unskinned
    // mesh: the shader sees zero weights and skips the skinning path.
    let joints = read_joints(reader.read_joints(0));
    let weights = read_weights(reader.read_weights(0));

    let indices: Vec<u32> = match reader.read_indices() {
        Some(gltf::mesh::util::ReadIndices::U8(iter)) => iter.map(u32::from).collect(),
        Some(gltf::mesh::util::ReadIndices::U16(iter)) => iter.map(u32::from).collect(),
        Some(gltf::mesh::util::ReadIndices::U32(iter)) => iter.collect(),
        None => (0..positions.len() as u32).collect(),
    };

    match mode {
        Mode::Points => None,
        Mode::Triangles => Some(triangle_mesh(
            &positions, normals, uvs, colors, &joints, &weights, &indices,
        )),
        Mode::TriangleStrip => {
            let mut triangles = Vec::with_capacity(indices.len().saturating_sub(2) * 3);
            for i in 0..indices.len().saturating_sub(2) {
                if i % 2 == 0 {
                    triangles.extend_from_slice(&[indices[i], indices[i + 1], indices[i + 2]]);
                } else {
                    triangles.extend_from_slice(&[indices[i + 1], indices[i], indices[i + 2]]);
                }
            }
            Some(triangle_mesh(
                &positions, normals, uvs, colors, &joints, &weights, &triangles,
            ))
        }
        Mode::TriangleFan => {
            let mut triangles = Vec::with_capacity(indices.len().saturating_sub(2) * 3);
            for i in 1..indices.len().saturating_sub(1) {
                triangles.extend_from_slice(&[indices[0], indices[i], indices[i + 1]]);
            }
            Some(triangle_mesh(
                &positions, normals, uvs, colors, &joints, &weights, &triangles,
            ))
        }
        Mode::Lines => Some(line_mesh(
            &positions, normals, uvs, colors, &joints, &weights, &indices,
        )),
        Mode::LineStrip => {
            let mut lines = Vec::with_capacity(indices.len().saturating_sub(1) * 2);
            for pair in indices.windows(2) {
                lines.extend_from_slice(pair);
            }
            Some(line_mesh(
                &positions, normals, uvs, colors, &joints, &weights, &lines,
            ))
        }
        Mode::LineLoop => {
            let mut lines = Vec::with_capacity(indices.len() * 2);
            for pair in indices.windows(2) {
                lines.extend_from_slice(pair);
            }
            if let (Some(first), Some(last)) = (indices.first(), indices.last()) {
                lines.extend_from_slice(&[*last, *first]);
            }
            Some(line_mesh(
                &positions, normals, uvs, colors, &joints, &weights, &lines,
            ))
        }
    }
}

fn triangle_mesh(
    positions: &[[f32; 3]],
    normals: Option<Vec<[f32; 3]>>,
    uvs: Option<Vec<[f32; 2]>>,
    colors: Option<Vec<[f32; 4]>>,
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    indices: &[u32],
) -> Mesh {
    match normals {
        Some(normals) => {
            let vertices = positions
                .iter()
                .enumerate()
                .map(|(index, position)| {
                    let uv = uvs
                        .as_ref()
                        .and_then(|uvs| uvs.get(index))
                        .copied()
                        .unwrap_or([0.0, 0.0]);
                    let color = colors
                        .as_ref()
                        .and_then(|colors| colors.get(index))
                        .copied()
                        .unwrap_or([1.0; 4]);
                    let normal = normals
                        .get(index)
                        .map(|normal| safe_normalize(Vec3::from_array(*normal)))
                        .unwrap_or(Vec3::Y);
                    vertex_from(index, joints, weights, *position, normal, color, uv)
                })
                .collect();

            Mesh::from_vertices(vertices, indices.to_vec())
        }
        None => {
            // glTF says a primitive without normals should be shaded flat, so
            // each triangle gets its own three vertices.
            let mut vertices = Vec::with_capacity(indices.len());
            let mut flat_indices = Vec::with_capacity(indices.len());

            for triangle in indices.chunks_exact(3) {
                let normal = triangle_normal(positions, triangle);
                for index in triangle {
                    let index = *index as usize;
                    let Some(position) = positions.get(index) else {
                        continue;
                    };
                    let uv = uvs
                        .as_ref()
                        .and_then(|uvs| uvs.get(index))
                        .copied()
                        .unwrap_or([0.0, 0.0]);
                    let color = colors
                        .as_ref()
                        .and_then(|colors| colors.get(index))
                        .copied()
                        .unwrap_or([1.0; 4]);
                    flat_indices.push(vertices.len() as u32);
                    vertices.push(vertex_from(
                        index, joints, weights, *position, normal, color, uv,
                    ));
                }
            }

            Mesh::from_vertices(vertices, flat_indices)
        }
    }
}

fn line_mesh(
    positions: &[[f32; 3]],
    normals: Option<Vec<[f32; 3]>>,
    uvs: Option<Vec<[f32; 2]>>,
    colors: Option<Vec<[f32; 4]>>,
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    indices: &[u32],
) -> Mesh {
    let vertices = positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let uv = uvs
                .as_ref()
                .and_then(|uvs| uvs.get(index))
                .copied()
                .unwrap_or([0.0, 0.0]);
            let color = colors
                .as_ref()
                .and_then(|colors| colors.get(index))
                .copied()
                .unwrap_or([1.0; 4]);
            let normal = normals
                .as_ref()
                .and_then(|normals| normals.get(index))
                .map(|normal| safe_normalize(Vec3::from_array(*normal)))
                .unwrap_or(Vec3::Y);
            vertex_from(index, joints, weights, *position, normal, color, uv)
        })
        .collect();

    Mesh {
        vertices,
        indices: indices.to_vec(),
        topology: wgpu::PrimitiveTopology::LineList,
    }
}

fn vertex_from(
    index: usize,
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    position: [f32; 3],
    normal: Vec3,
    color: [f32; 4],
    uv: [f32; 2],
) -> Vertex {
    Vertex {
        position,
        normal: normal.to_array(),
        color,
        uv,
        joints: joints.get(index).copied().unwrap_or([0; 4]),
        weights: weights.get(index).copied().unwrap_or([0.0; 4]),
    }
}

fn read_joints(joints: Option<gltf::mesh::util::ReadJoints<'_>>) -> Vec<[u16; 4]> {
    use gltf::mesh::util::ReadJoints;

    match joints {
        Some(ReadJoints::U8(iter)) => iter
            .map(|joint| {
                [
                    u16::from(joint[0]),
                    u16::from(joint[1]),
                    u16::from(joint[2]),
                    u16::from(joint[3]),
                ]
            })
            .collect(),
        Some(ReadJoints::U16(iter)) => iter.collect(),
        None => Vec::new(),
    }
}

fn read_weights(weights: Option<gltf::mesh::util::ReadWeights<'_>>) -> Vec<[f32; 4]> {
    use gltf::mesh::util::ReadWeights;

    match weights {
        Some(ReadWeights::U8(iter)) => iter
            .map(|weight| {
                [
                    f32::from(weight[0]) / 255.0,
                    f32::from(weight[1]) / 255.0,
                    f32::from(weight[2]) / 255.0,
                    f32::from(weight[3]) / 255.0,
                ]
            })
            .collect(),
        Some(ReadWeights::U16(iter)) => iter
            .map(|weight| {
                [
                    f32::from(weight[0]) / 65535.0,
                    f32::from(weight[1]) / 65535.0,
                    f32::from(weight[2]) / 65535.0,
                    f32::from(weight[3]) / 65535.0,
                ]
            })
            .collect(),
        Some(ReadWeights::F32(iter)) => iter.collect(),
        None => Vec::new(),
    }
}

/// Reads one animation clip into a flat, reusable set of sampled channels.
fn read_animation(
    animation: &gltf::Animation<'_>,
    buffers: &[gltf::buffer::Data],
) -> GltfAnimation {
    let mut channels = Vec::new();
    let mut duration: f32 = 0.0;

    for channel in animation.channels() {
        let target = channel.target();
        let property = match target.property() {
            gltf::animation::Property::Translation => GltfAnimationProperty::Translation,
            gltf::animation::Property::Rotation => GltfAnimationProperty::Rotation,
            gltf::animation::Property::Scale => GltfAnimationProperty::Scale,
            // Morph targets are not part of the vertex format yet.
            gltf::animation::Property::MorphTargetWeights => continue,
        };
        let interpolation = match channel.sampler().interpolation() {
            gltf::animation::Interpolation::Linear => GltfInterpolation::Linear,
            gltf::animation::Interpolation::Step => GltfInterpolation::Step,
            gltf::animation::Interpolation::CubicSpline => GltfInterpolation::CubicSpline,
        };

        let reader = channel.reader(|buffer| buffers.get(buffer.index()).map(|data| &data.0[..]));
        let Some(times) = reader
            .read_inputs()
            .map(|inputs| inputs.collect::<Vec<f32>>())
        else {
            continue;
        };
        let mut values: Vec<[f32; 4]> = match reader.read_outputs() {
            Some(gltf::animation::util::ReadOutputs::Translations(outputs)) => outputs
                .map(|value| [value[0], value[1], value[2], 0.0])
                .collect(),
            Some(gltf::animation::util::ReadOutputs::Scales(outputs)) => outputs
                .map(|value| [value[0], value[1], value[2], 0.0])
                .collect(),
            Some(gltf::animation::util::ReadOutputs::Rotations(outputs)) => {
                read_rotation_samples(outputs)
            }
            _ => continue,
        };

        // A cubic spline stores (in-tangent, value, out-tangent) per keyframe.
        // Ignoring the tangents and keeping the middle value gives a smooth
        // enough approximation for the linear sampler in the player.
        if interpolation == GltfInterpolation::CubicSpline {
            values = values
                .chunks(3)
                .filter_map(|triple| triple.get(1).copied())
                .collect();
        }

        let samples: Vec<(f32, [f32; 4])> = times.into_iter().zip(values).collect();
        if let Some(&(last, _)) = samples.last() {
            duration = duration.max(last);
        }
        if !samples.is_empty() {
            channels.push(GltfAnimationChannel {
                node: target.node().index(),
                property,
                interpolation,
                samples,
            });
        }
    }

    GltfAnimation { channels, duration }
}

/// Converts the integer or float rotation formats allowed by glTF into XYZW.
fn read_rotation_samples(rotations: gltf::animation::util::Rotations<'_>) -> Vec<[f32; 4]> {
    use gltf::animation::util::Rotations;

    match rotations {
        Rotations::I8(iter) => iter
            .map(|r| {
                [
                    f32::from(r[0]) / 127.0,
                    f32::from(r[1]) / 127.0,
                    f32::from(r[2]) / 127.0,
                    f32::from(r[3]) / 127.0,
                ]
            })
            .collect(),
        Rotations::U8(iter) => iter
            .map(|r| {
                [
                    f32::from(r[0]) / 255.0 * 2.0 - 1.0,
                    f32::from(r[1]) / 255.0 * 2.0 - 1.0,
                    f32::from(r[2]) / 255.0 * 2.0 - 1.0,
                    f32::from(r[3]) / 255.0 * 2.0 - 1.0,
                ]
            })
            .collect(),
        Rotations::I16(iter) => iter
            .map(|r| {
                [
                    f32::from(r[0]) / 32767.0,
                    f32::from(r[1]) / 32767.0,
                    f32::from(r[2]) / 32767.0,
                    f32::from(r[3]) / 32767.0,
                ]
            })
            .collect(),
        Rotations::U16(iter) => iter
            .map(|r| {
                [
                    f32::from(r[0]) / 65535.0 * 2.0 - 1.0,
                    f32::from(r[1]) / 65535.0 * 2.0 - 1.0,
                    f32::from(r[2]) / 65535.0 * 2.0 - 1.0,
                    f32::from(r[3]) / 65535.0 * 2.0 - 1.0,
                ]
            })
            .collect(),
        Rotations::F32(iter) => iter.collect(),
    }
}

fn triangle_normal(positions: &[[f32; 3]], triangle: &[u32]) -> Vec3 {
    let Some(&a) = triangle.first() else {
        return Vec3::Y;
    };
    let Some(&b) = triangle.get(1) else {
        return Vec3::Y;
    };
    let Some(&c) = triangle.get(2) else {
        return Vec3::Y;
    };

    let Some(a) = positions.get(a as usize) else {
        return Vec3::Y;
    };
    let Some(b) = positions.get(b as usize) else {
        return Vec3::Y;
    };
    let Some(c) = positions.get(c as usize) else {
        return Vec3::Y;
    };

    safe_normalize(
        (Vec3::from_array(*b) - Vec3::from_array(*a))
            .cross(Vec3::from_array(*c) - Vec3::from_array(*a)),
    )
}

/// Normalizes a vector, falling back to up for a degenerate one.
fn safe_normalize(vector: Vec3) -> Vec3 {
    let normalized = vector.normalize_or_zero();

    if normalized == Vec3::ZERO {
        Vec3::Y
    } else {
        normalized
    }
}

fn read_tex_coords(iter: gltf::mesh::util::ReadTexCoords<'_>) -> Vec<[f32; 2]> {
    use gltf::mesh::util::ReadTexCoords;

    match iter {
        ReadTexCoords::U8(iter) => iter
            .map(|uv| [uv[0] as f32 / 255.0, uv[1] as f32 / 255.0])
            .collect(),
        ReadTexCoords::U16(iter) => iter
            .map(|uv| [uv[0] as f32 / 65535.0, uv[1] as f32 / 65535.0])
            .collect(),
        ReadTexCoords::F32(iter) => iter.collect(),
    }
}

fn read_colors(iter: gltf::mesh::util::ReadColors<'_>) -> Vec<[f32; 4]> {
    use gltf::mesh::util::ReadColors;

    match iter {
        ReadColors::RgbU8(iter) => iter
            .map(|c| {
                [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                    1.0,
                ]
            })
            .collect(),
        ReadColors::RgbaU8(iter) => iter
            .map(|c| {
                [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                    c[3] as f32 / 255.0,
                ]
            })
            .collect(),
        ReadColors::RgbU16(iter) => iter
            .map(|c| {
                [
                    c[0] as f32 / 65535.0,
                    c[1] as f32 / 65535.0,
                    c[2] as f32 / 65535.0,
                    1.0,
                ]
            })
            .collect(),
        ReadColors::RgbaU16(iter) => iter
            .map(|c| {
                [
                    c[0] as f32 / 65535.0,
                    c[1] as f32 / 65535.0,
                    c[2] as f32 / 65535.0,
                    c[3] as f32 / 65535.0,
                ]
            })
            .collect(),
        ReadColors::RgbF32(iter) => iter.map(|c| [c[0], c[1], c[2], 1.0]).collect(),
        ReadColors::RgbaF32(iter) => iter.collect(),
    }
}

fn image_to_rgba8(image: &gltf::image::Data) -> GltfImage {
    let pixels = match image.format {
        gltf::image::Format::R8 => greyscale(&image.pixels, 1),
        gltf::image::Format::R8G8 => greyscale_alpha(&image.pixels),
        gltf::image::Format::R8G8B8 => rgb(&image.pixels, 3),
        gltf::image::Format::R8G8B8A8 => image.pixels.clone(),
        gltf::image::Format::R16 => greyscale_u16(&image.pixels, 1),
        gltf::image::Format::R16G16 => greyscale_alpha_u16(&image.pixels, 2),
        gltf::image::Format::R16G16B16 => rgb_u16(&image.pixels, 3),
        gltf::image::Format::R16G16B16A16 => rgba_u16(&image.pixels, 4),
        gltf::image::Format::R32G32B32FLOAT => rgb_f32(&image.pixels, 3),
        gltf::image::Format::R32G32B32A32FLOAT => rgba_f32(&image.pixels, 4),
    };

    GltfImage {
        width: image.width,
        height: image.height,
        rgba8: pixels,
    }
}

fn greyscale(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels * 4);
    for pixel in pixels.chunks_exact(channels) {
        let value = pixel[0];
        out.extend_from_slice(&[value, value, value, 255]);
    }
    out
}

fn greyscale_alpha(pixels: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() * 2);
    for pixel in pixels.chunks_exact(2) {
        out.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
    }
    out
}

fn greyscale_u16(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels * 4);
    for pixel in pixels.chunks_exact(channels * 2) {
        let value = (u16::from_ne_bytes([pixel[0], pixel[1]]) >> 8) as u8;
        out.extend_from_slice(&[value, value, value, 255]);
    }
    out
}

fn greyscale_alpha_u16(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels * 4);
    for pixel in pixels.chunks_exact(channels * 2) {
        let value = (u16::from_ne_bytes([pixel[0], pixel[1]]) >> 8) as u8;
        let alpha = (u16::from_ne_bytes([pixel[2], pixel[3]]) >> 8) as u8;
        out.extend_from_slice(&[value, value, value, alpha]);
    }
    out
}

fn rgb(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels * 4);
    for pixel in pixels.chunks_exact(channels) {
        out.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
    }
    out
}

fn rgb_u16(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels * 4);
    for pixel in pixels.chunks_exact(channels * 2) {
        out.extend_from_slice(&[
            (u16::from_ne_bytes([pixel[0], pixel[1]]) >> 8) as u8,
            (u16::from_ne_bytes([pixel[2], pixel[3]]) >> 8) as u8,
            (u16::from_ne_bytes([pixel[4], pixel[5]]) >> 8) as u8,
            255,
        ]);
    }
    out
}

fn rgba_u16(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels * 4);
    for pixel in pixels.chunks_exact(channels * 2) {
        out.extend_from_slice(&[
            (u16::from_ne_bytes([pixel[0], pixel[1]]) >> 8) as u8,
            (u16::from_ne_bytes([pixel[2], pixel[3]]) >> 8) as u8,
            (u16::from_ne_bytes([pixel[4], pixel[5]]) >> 8) as u8,
            (u16::from_ne_bytes([pixel[6], pixel[7]]) >> 8) as u8,
        ]);
    }
    out
}

fn rgb_f32(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels * 4);
    for pixel in pixels.chunks_exact(channels * 4) {
        out.extend_from_slice(&[
            float_to_u8(&pixel[0..4]),
            float_to_u8(&pixel[4..8]),
            float_to_u8(&pixel[8..12]),
            255,
        ]);
    }
    out
}

fn rgba_f32(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels * 4);
    for pixel in pixels.chunks_exact(channels * 4) {
        out.extend_from_slice(&[
            float_to_u8(&pixel[0..4]),
            float_to_u8(&pixel[4..8]),
            float_to_u8(&pixel[8..12]),
            float_to_u8(&pixel[12..16]),
        ]);
    }
    out
}

fn float_to_u8(bytes: &[u8]) -> u8 {
    let value = f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    // Three positions, packed as little-endian f32 and embedded as a data URI:
    // (0,0,0), (1,0,0), (0,1,0).
    const TRIANGLE: &str = r#"{
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
        "accessors": [{
            "bufferView": 0,
            "componentType": 5126,
            "count": 3,
            "type": "VEC3",
            "min": [0.0, 0.0, 0.0],
            "max": [1.0, 1.0, 0.0]
        }],
        "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}],
        "buffers": [{
            "byteLength": 36,
            "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAA"
        }]
    }"#;

    #[test]
    fn a_triangle_becomes_one_primitive_and_one_instance() {
        let model = GltfModel::from_slice(TRIANGLE).unwrap();

        assert_eq!(model.primitives.len(), 1);
        assert_eq!(model.instances.len(), 1);
        assert_eq!(model.instances[0].primitive, 0);
        assert_eq!(model.primitives[0].mesh.indices, vec![0, 1, 2]);
        assert_eq!(model.primitives[0].mesh.vertices.len(), 3);
    }

    #[test]
    fn missing_normals_are_computed_per_face() {
        let model = GltfModel::from_slice(TRIANGLE).unwrap();
        let normal = Vec3::from_array(model.primitives[0].mesh.vertices[0].normal);

        assert!((normal - Vec3::Z).length() < 1e-6);
    }

    #[test]
    fn a_node_transform_becomes_the_instance_transform() {
        let source = TRIANGLE.replace(
            r#""nodes": [{"mesh": 0}]"#,
            r#""nodes": [{"mesh": 0, "translation": [1.0, 2.0, 3.0]}]"#,
        );
        let model = GltfModel::from_slice(source).unwrap();
        let translation = model.instances[0].transform.w_axis.truncate();

        assert!((translation - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6);
    }

    #[test]
    fn a_triangle_strip_is_expanded_to_a_triangle_list() {
        // A quad as a strip: positions (0,0), (1,0), (1,1), (0,1) and indices
        // 0, 1, 2, 3, all in one buffer.
        let source = r#"{
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{
                "attributes": {"POSITION": 0},
                "indices": 1,
                "mode": 5
            }]}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
                 "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]},
                {"bufferView": 1, "componentType": 5123, "count": 4, "type": "SCALAR"}
            ],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 48},
                {"buffer": 0, "byteOffset": 48, "byteLength": 8}
            ],
            "buffers": [{
                "byteLength": 56,
                "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAACAPwAAgD8AAAAAAAAAAAAAgD8AAAAAAAABAAIAAwA="
            }]
        }"#;

        let model = GltfModel::from_slice(source).unwrap();
        let vertices = &model.primitives[0].mesh.vertices;

        // The second triangle is (2, 1, 3): the strip alternates the winding
        // order, and flat shading gives every corner its own vertex.
        assert_eq!(vertices[3].position, [1.0, 1.0, 0.0]);
        assert_eq!(vertices[4].position, [1.0, 0.0, 0.0]);
        assert_eq!(vertices[5].position, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn an_embedded_texture_is_decoded_from_a_json_slice() {
        let source = TRIANGLE
            .replace(
                r#""meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],"#,
                r#""meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "material": 0}]}],"#,
            )
            .replace(
                r#""buffers": ["#,
                r#""materials": [{"pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}}],
                   "textures": [{"source": 0}],
                   "images": [{"uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="}],
                   "buffers": ["#,
            );

        let model = GltfModel::from_slice(source).unwrap();

        assert_eq!(model.materials[0].base_color_texture, Some(0));
        assert_eq!(model.images.len(), 1);
        assert_eq!((model.images[0].width, model.images[0].height), (1, 1));
        assert_eq!(model.images[0].rgba8.len(), 4);
    }

    #[test]
    fn a_scene_without_meshes_is_an_error() {
        let source = TRIANGLE
            .replace(r#""nodes": [{"mesh": 0}]"#, r#""nodes": [{}]"#)
            .replace(
                r#""meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}]"#,
                r#""meshes": []"#,
            );

        assert!(matches!(
            GltfModel::from_slice(source),
            Err(GltfError::Empty)
        ));
    }

    /// Wraps JSON and binary data in a GLB, so no base64 helper is needed.
    fn glb(json: &str, binary: &[u8]) -> Vec<u8> {
        let json = json.as_bytes();
        let json_len = (json.len() + 3) & !3;
        let bin_len = (binary.len() + 3) & !3;
        let total = 12 + 8 + json_len + 8 + bin_len;

        let mut bytes = Vec::with_capacity(total);
        bytes.extend_from_slice(b"glTF");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&(total as u32).to_le_bytes());
        bytes.extend_from_slice(&(json_len as u32).to_le_bytes());
        bytes.extend_from_slice(b"JSON");
        bytes.extend_from_slice(json);
        bytes.resize(bytes.len() + json_len - json.len(), b' ');
        bytes.extend_from_slice(&(bin_len as u32).to_le_bytes());
        bytes.extend_from_slice(b"BIN\0");
        bytes.extend_from_slice(binary);
        bytes.resize(bytes.len() + bin_len - binary.len(), 0);
        bytes
    }

    fn push_f32(data: &mut Vec<u8>, values: &[f32]) {
        for value in values {
            data.extend_from_slice(&value.to_le_bytes());
        }
    }

    fn push_u16(data: &mut Vec<u8>, values: &[u16]) {
        for value in values {
            data.extend_from_slice(&value.to_le_bytes());
        }
    }

    /// One triangle fully bound to one joint, whose translation is animated.
    fn skinned_glb() -> Vec<u8> {
        let mut data = Vec::new();
        push_f32(&mut data, &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        push_u16(&mut data, &[0; 12]);
        push_f32(
            &mut data,
            &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        );
        push_f32(
            &mut data,
            &[
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        );
        push_f32(&mut data, &[0.0, 1.0]);
        push_f32(&mut data, &[0.0, 0.0, 0.0, 2.0, 0.0, 0.0]);
        assert_eq!(data.len(), 204);

        let json = r#"{
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0, 1]}],
            "nodes": [
                {"translation": [0.0, 0.0, 0.0]},
                {"mesh": 0, "skin": 0}
            ],
            "meshes": [{"primitives": [{
                "attributes": {"POSITION": 0, "JOINTS_0": 1, "WEIGHTS_0": 2}
            }]}],
            "skins": [{"joints": [0], "inverseBindMatrices": 3}],
            "animations": [{
                "channels": [{
                    "sampler": 0,
                    "target": {"node": 0, "path": "translation"}
                }],
                "samplers": [{"input": 4, "output": 5}]
            }],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                 "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]},
                {"bufferView": 1, "componentType": 5123, "count": 3, "type": "VEC4"},
                {"bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC4"},
                {"bufferView": 3, "componentType": 5126, "count": 1, "type": "MAT4"},
                {"bufferView": 4, "componentType": 5126, "count": 2, "type": "SCALAR",
                 "min": [0.0], "max": [1.0]},
                {"bufferView": 5, "componentType": 5126, "count": 2, "type": "VEC3"}
            ],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 24},
                {"buffer": 0, "byteOffset": 60, "byteLength": 48},
                {"buffer": 0, "byteOffset": 108, "byteLength": 64},
                {"buffer": 0, "byteOffset": 172, "byteLength": 8},
                {"buffer": 0, "byteOffset": 180, "byteLength": 24}
            ],
            "buffers": [{"byteLength": 204}]
        }"#;
        glb(json, &data)
    }

    #[test]
    fn joints_weights_skins_and_animations_survive_loading() {
        let model = GltfModel::from_slice(skinned_glb()).unwrap();

        assert_eq!(model.nodes.len(), 2);
        assert_eq!(model.instances[0].node, 1);
        assert_eq!(model.instances[0].skin, Some(0));
        assert_eq!(model.skins[0].joints, vec![0]);
        assert_eq!(model.skins[0].inverse_bind_matrices.len(), 1);

        let vertex = &model.primitives[0].mesh.vertices[0];
        assert_eq!(vertex.joints, [0; 4]);
        assert_eq!(vertex.weights, [1.0, 0.0, 0.0, 0.0]);

        assert_eq!(model.animations.len(), 1);
        assert_eq!(model.animations[0].duration, 1.0);
        assert_eq!(model.animations[0].channels.len(), 1);
        assert_eq!(
            model.animations[0].channels[0].property,
            GltfAnimationProperty::Translation
        );
    }

    #[test]
    fn the_skinned_demo_asset_loads() {
        let model = GltfModel::from_file("examples/wgpu/assets/skinned.glb").unwrap();

        assert_eq!(model.skins.len(), 1);
        assert_eq!(model.skins[0].joints.len(), 2);
        assert_eq!(model.animations.len(), 1);
        assert_eq!(model.animations[0].channels.len(), 1);
        assert!(model.primitives[0]
            .mesh
            .vertices
            .iter()
            .any(|vertex| vertex.joints[0] == 1));
    }

    #[test]
    fn an_animated_joint_moves_the_bind_pose() {
        use crate::gpu::GltfAnimationPlayer;

        let model = GltfModel::from_slice(skinned_glb()).unwrap();
        let mut player = GltfAnimationPlayer::new(&model);
        assert_eq!(player.duration(), 1.0);
        assert!(player.joint_matrices(0).unwrap()[0].w_axis.x.abs() < 1e-6);

        player.set_time(0.5);
        let matrices = player.joint_matrices(0).unwrap();
        assert!((matrices[0].w_axis.x - 1.0).abs() < 1e-6);
    }
}
