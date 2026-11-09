// The one shader behind every built-in pipeline.
//
// Three bind groups: per-camera data in group 0, per-draw data in group 1 and
// an optional albedo texture in group 2. The draw group uses a dynamic offset,
// so one buffer holds every queued draw and the renderer only changes the
// offset between them. Only the textured pipelines reference group 2; the
// untextured entry points leave it out of their pipeline layout.
//
// Colours arrive already in linear space. When the swapchain format is sRGB,
// wgpu converts them on write, which is why the fragment shaders do no gamma
// work of their own.

const MAX_LIGHTS: u32 = 8u;

struct Camera {
    view_proj: mat4x4<f32>,
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

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let world = draw.model * vec4<f32>(input.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world;
    out.world_position = world.xyz;
    // A model matrix may rotate and scale uniformly; that is all the built-in
    // draws do, so rotating the normal by it is enough.
    out.normal = normalize((draw.model * vec4<f32>(input.normal, 0.0)).xyz);
    out.color = input.color * draw.color;
    out.uv = input.uv;
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
