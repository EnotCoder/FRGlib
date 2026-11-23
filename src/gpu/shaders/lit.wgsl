// The one shader behind every built-in pipeline.
//
// Three bind groups: per-camera data in group 0, per-draw data and joint
// matrices in group 1, and an optional albedo texture in group 2 (group 3
// belongs to the skybox). Both group 1 bindings use dynamic offsets, so one
// buffer each holds every queued draw and the renderer only changes the offsets
// between them. Only the textured pipelines reference group 2; the untextured
// entry points leave it out of their pipeline layout.
//
// Colours arrive already in linear space. When the swapchain format is sRGB,
// wgpu converts them on write, which is why the fragment shaders do no gamma
// work of their own.

const MAX_LIGHTS: u32 = 8u;
// Must match `gltf::MAX_JOINTS` on the Rust side: one slot in the skin buffer
// always holds this many mat4s, even for smaller skins.
const MAX_JOINTS: u32 = 64u;

struct Camera {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    // rgb is linear colour, a is intensity.
    ambient: vec4<f32>,
    // x is the number of active lights; the rest pads the vec4.
    light_count: vec4<u32>,
    // xyz is the direction (directional) or position (point); w is 0 or 1.
    light_position: array<vec4<f32>, MAX_LIGHTS>,
    // rgb is linear colour, a is intensity.
    light_color: array<vec4<f32>, MAX_LIGHTS>,
    // x is the range of a point light.
    light_params: array<vec4<f32>, MAX_LIGHTS>,
};

struct Draw {
    model: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> draw: Draw;

@group(2) @binding(0)
var albedo: texture_2d<f32>;

@group(2) @binding(1)
var albedo_sampler: sampler;

// Read-only joint matrices, alongside the per-draw uniform in group 1. Draws
// without skinning point at a slot of identity matrices and have zero weights,
// so their vertices take the early exit in `skin_vertex`.
@group(1) @binding(1)
var<storage, read> joint_matrices: array<mat4x4<f32>>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct SkinnedVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
};

// Up to four linear blend skinning influences. Weights are normalised here so
// glTF's unnormalised U8/U16 weights work too.
fn skin_vertex(
    position: vec3<f32>,
    normal_in: vec3<f32>,
    joints: vec4<u32>,
    weights: vec4<f32>,
) -> SkinnedVertex {
    var result: SkinnedVertex;
    result.position = position;
    result.normal = normal_in;

    let total = weights.x + weights.y + weights.z + weights.w;
    if (total < 0.00001) {
        return result;
    }

    var position_sum = vec4<f32>(0.0);
    var normal_sum = vec4<f32>(0.0);
    for (var i = 0u; i < 4u; i = i + 1u) {
        let weight = weights[i];
        if (weight > 0.0) {
            let matrix = joint_matrices[min(joints[i], MAX_JOINTS - 1u)];
            position_sum += matrix * vec4<f32>(position, 1.0) * weight;
            normal_sum += matrix * vec4<f32>(normal_in, 0.0) * weight;
        }
    }

    result.position = position_sum.xyz / total;
    result.normal = normalize(normal_sum.xyz);
    return result;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let skinned = skin_vertex(input.position, input.normal, input.joints, input.weights);
    let world = draw.model * vec4<f32>(skinned.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world;
    out.world_position = world.xyz;
    // A model matrix may rotate and scale uniformly; that is all the built-in
    // draws do, so rotating the normal by it is enough.
    out.normal = normalize((draw.model * vec4<f32>(skinned.normal, 0.0)).xyz);
    out.color = input.color * draw.color;
    out.uv = input.uv;
    return out;
}

struct SkyboxOutput {
    @builtin(position) clip_position: vec4<f32>,
    // Normalised device coordinates; w is always 1 for this triangle.
    @location(0) ndc: vec2<f32>,
};

@group(3) @binding(0)
var skybox_texture: texture_cube<f32>;

@group(3) @binding(1)
var skybox_sampler: sampler;

// One oversized triangle covers the viewport without a vertex buffer. The ray
// is reconstructed per fragment so the interpolated NDC stays linear.
@vertex
fn vs_skybox(@builtin(vertex_index) index: u32) -> SkyboxOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[index];

    var out: SkyboxOutput;
    out.clip_position = vec4<f32>(position, 1.0, 1.0);
    out.ndc = position;
    return out;
}

// Every light in the scene plus a little ambient and specular. A small ambient
// term keeps the side facing away from all lights readable instead of black.
fn shade(world_position: vec3<f32>, normal_in: vec3<f32>, base: vec4<f32>) -> vec4<f32> {
    let normal = normalize(normal_in);
    let view_dir = normalize(camera.camera_pos.xyz - world_position);

    var diffuse = vec3<f32>(0.0);
    var specular = vec3<f32>(0.0);
    let count = min(camera.light_count.x, MAX_LIGHTS);

    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let placement = camera.light_position[i];
        let color = camera.light_color[i];

        var to_light: vec3<f32>;
        var attenuation = 1.0;

        if (placement.w < 0.5) {
            // A directional light stores the direction it travels in.
            to_light = normalize(-placement.xyz);
        } else {
            let delta = placement.xyz - world_position;
            let distance_sq = max(dot(delta, delta), 0.000001);
            let distance = sqrt(distance_sq);
            to_light = delta / distance;
            let range = max(camera.light_params[i].x, 0.0001);
            attenuation = 1.0 / (1.0 + distance_sq / (range * range));
        }

        let radiance = color.rgb * color.a * attenuation;
        diffuse += radiance * max(dot(normal, to_light), 0.0);

        let half_vector = normalize(to_light + view_dir);
        let blinn = pow(max(dot(normal, half_vector), 0.0), 32.0);
        specular += radiance * blinn * 0.25;
    }

    let lit = base.rgb * camera.ambient.rgb * camera.ambient.a
        + base.rgb * diffuse * (1.0 - camera.ambient.rgb * camera.ambient.a)
        + specular;

    return vec4<f32>(lit, base.a);
}

@fragment
fn fs_skybox(input: SkyboxOutput) -> @location(0) vec4<f32> {
    let far = camera.inv_view_proj * vec4<f32>(input.ndc, 1.0, 1.0);
    let direction = far.xyz / far.w - camera.camera_pos.xyz;
    return textureSample(skybox_texture, skybox_sampler, normalize(direction));
}

@fragment
fn fs_lit(input: VertexOutput) -> @location(0) vec4<f32> {
    return shade(input.world_position, input.normal, input.color);
}

@fragment
fn fs_unlit(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

@fragment
fn fs_lit_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo, albedo_sampler, input.uv);
    return shade(input.world_position, input.normal, texel * input.color);
}

@fragment
fn fs_unlit_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(albedo, albedo_sampler, input.uv) * input.color;
}
