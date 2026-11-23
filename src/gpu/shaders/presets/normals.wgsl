// Normals: a debug view. RGB encodes the interpolated world-space normal.

fn normal_color(input: VertexOutput) -> vec3<f32> {
    return normalize(input.normal) * 0.5 + vec3<f32>(0.5);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(normal_color(input), 1.0);
}

@fragment
fn fs_main_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo, albedo_sampler, input.uv);
    return vec4<f32>(mix(texel.rgb, normal_color(input), 0.65), input.color.a);
}
