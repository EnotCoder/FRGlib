// Comic: posterised ink colours, printed halftone dots and drawn contours.

fn comic_shade(input: VertexOutput, base: vec4<f32>) -> vec4<f32> {
    let normal = normalize(input.normal);
    let key = key_light(input.world_position);
    let ndl = max(dot(normal, key.direction), 0.0);

    let flat = posterize(base.rgb * (ambient_light() + key.radiance * ndl), 4.0);

    // Dots grow in the shadows, like an old printed comic book.
    let grid = fract(input.clip_position.xy / 7.0) - vec2<f32>(0.5);
    let radius = 0.16 + 0.30 * (1.0 - clamp(luma(flat), 0.0, 1.0));
    let halftone = select(1.0, 0.55, length(grid) < radius);

    // A sharp change in the interpolated normal means a silhouette or a fold.
    let bend = length(fwidth(normal));
    let outline = select(1.0, 0.10, bend > 0.28);

    return vec4<f32>(flat * halftone * outline, base.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return comic_shade(input, input.color);
}

@fragment
fn fs_main_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo, albedo_sampler, input.uv);
    return comic_shade(input, texel * input.color);
}
