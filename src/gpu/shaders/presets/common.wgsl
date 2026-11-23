// Shared vertex stage and helpers for the built-in shader presets.
//
// `Renderer3D::create_material` prepends the header of `lit.wgsl` (structs,
// bindings and `VertexOutput`) to every preset source, then appends this file
// followed by the preset itself. Keeping the plumbing here means a preset only
// has to describe its look in `fs_main` / `fs_main_textured`.

// `skin_vertex` and the skin bind group come from the `lit.wgsl` header that
// `create_material` prepends, so skinned glTF models work with every style.
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let skinned = skin_vertex(input.position, input.normal, input.joints, input.weights);
    let world = draw.model * vec4<f32>(skinned.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world;
    out.world_position = world.xyz;
    // Same assumption as the built-in shader: uniform scale only.
    out.normal = normalize((draw.model * vec4<f32>(skinned.normal, 0.0)).xyz);
    out.color = input.color * draw.color;
    out.uv = input.uv;
    return out;
}

struct LightSample {
    direction: vec3<f32>,
    radiance: vec3<f32>,
};

// The presets are stylised: one key light is enough, and it keeps the WGSL
// short. Point lights get the same inverse-square falloff as `lit.wgsl`.
fn key_light(world_position: vec3<f32>) -> LightSample {
    var result: LightSample;
    result.direction = vec3<f32>(0.4, 0.8, 0.3);
    result.radiance = vec3<f32>(0.0);

    let count = min(camera.light_count.x, MAX_LIGHTS);
    if (count > 0u) {
        let placement = camera.light_position[0];
        let color = camera.light_color[0];
        if (placement.w < 0.5) {
            result.direction = normalize(-placement.xyz);
            result.radiance = color.rgb * color.a;
        } else {
            let delta = placement.xyz - world_position;
            let distance = max(length(delta), 0.0001);
            result.direction = delta / distance;
            let range = max(camera.light_params[0].x, 0.0001);
            result.radiance = color.rgb * color.a
                / (1.0 + distance * distance / (range * range));
        }
    }
    return result;
}

fn view_vector(world_position: vec3<f32>) -> vec3<f32> {
    return normalize(camera.camera_pos.xyz - world_position);
}

fn ambient_light() -> vec3<f32> {
    return camera.ambient.rgb * camera.ambient.a;
}

fn luma(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn posterize(color: vec3<f32>, levels: f32) -> vec3<f32> {
    return floor(color * levels + 0.5) / levels;
}

// Cheap deterministic noise, used for grain, dithering and flicker. There is
// no time uniform in the camera buffer, so animated-looking patterns are tied
// to world position instead of wall-clock time.
fn hash21(point: vec2<f32>) -> f32 {
    let q = fract(point * vec2<f32>(123.34, 456.21));
    return fract(dot(q, q + 45.32));
}
