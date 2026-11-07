// The one shader behind every built-in pipeline.
//
// Two bind groups, both uniform: per-camera data in group 0, per-draw data in
// group 1. The draw group uses a dynamic offset, so one buffer holds every
// queued draw and the renderer only changes the offset between them.
//
// Colours arrive already in linear space. When the swapchain format is sRGB,
// wgpu converts them on write, which is why the fragment shaders do no gamma
// work of their own.

struct Camera {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
};

struct Draw {
    model: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> draw: Draw;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
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
    return out;
}

@fragment
fn fs_lit(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let to_light = normalize(-camera.light_dir.xyz);
    let view_dir = normalize(camera.camera_pos.xyz - input.world_position);
    let half_vector = normalize(to_light + view_dir);

    let diffuse = max(dot(normal, to_light), 0.0);
    let specular = pow(max(dot(normal, half_vector), 0.0), 32.0);

    // A small ambient term keeps the side facing away from the light readable
    // instead of solid black.
    let ambient = 0.18;
    let lit = input.color.rgb * (ambient + (1.0 - ambient) * diffuse)
        + camera.light_color.rgb * specular * 0.25;

    return vec4<f32>(lit, input.color.a);
}

@fragment
fn fs_unlit(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
