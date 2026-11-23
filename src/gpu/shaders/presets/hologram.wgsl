// Hologram: a flickering projection made of scanlines, fresnel glow and alpha.
//
// Draw it like other custom materials (opaque queue, depth writes on). For a
// true see-through hologram, use `Material::Transparent` for the glow instead.

fn hologram_shade(input: VertexOutput, base: vec4<f32>) -> vec4<f32> {
    let normal = normalize(input.normal);
    let view = view_vector(input.world_position);
    let fresnel = pow(1.0 - abs(dot(normal, view)), 2.5);

    let scanline = 0.55 + 0.45 * step(0.35, fract(input.clip_position.y * 0.12));
    let flicker = 0.90 + 0.10 * hash21(vec2<f32>(
        floor(input.clip_position.y / 6.0), 0.0));
    let bands = 0.75 + 0.25 * sin(input.world_position.y * 24.0);

    let glow = base.rgb * (0.35 + fresnel * 1.6) * scanline * flicker * bands;
    let alpha = base.a * clamp(0.22 + fresnel * 0.75, 0.0, 1.0);
    return vec4<f32>(glow, alpha);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return hologram_shade(input, input.color);
}

@fragment
fn fs_main_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo, albedo_sampler, input.uv);
    return hologram_shade(input, texel * input.color);
}
