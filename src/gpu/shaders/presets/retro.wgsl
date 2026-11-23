// Retro: chunky sampled pixels, a small palette, CRT scanlines and fringes.

fn retro_shade(input: VertexOutput, base: vec4<f32>) -> vec4<f32> {
    let key = key_light(input.world_position);
    let ndl = max(dot(normalize(input.normal), key.direction), 0.0);
    let lit = base.rgb * (ambient_light() + key.radiance * ndl);

    // Eight levels per channel give the banding of an 8-bit palette.
    var color = posterize(lit, 7.0);

    // A CRT darkens every other group of scanlines.
    let scanline = 0.80 + 0.20 * step(0.5, fract(input.clip_position.y * 0.25));
    color *= scanline;

    // A tiny channel offset reads as a misaligned analogue image.
    color = mix(color, vec3<f32>(color.r * 1.10, color.g, color.b * 0.90), 0.35);

    let vignette = 1.0 - 0.35 * smoothstep(0.25, 0.75,
        length(input.uv - vec2<f32>(0.5)));
    color *= vignette;

    return vec4<f32>(color, base.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return retro_shade(input, input.color);
}

@fragment
fn fs_main_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    // Snap the lookup to a coarse grid: the texture itself becomes pixel art.
    let uv = floor(input.uv * 32.0) / 32.0;
    let texel = textureSample(albedo, albedo_sampler, uv);
    return retro_shade(input, texel * input.color);
}
