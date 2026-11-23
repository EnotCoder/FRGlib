// Pixel: ordered dithering and a tiny palette, like a 16-bit sprite.

fn pixel_shade(input: VertexOutput, base: vec4<f32>) -> vec4<f32> {
    let key = key_light(input.world_position);
    let ndl = max(dot(normalize(input.normal), key.direction), 0.0);
    let lit = base.rgb * (ambient_light() + key.radiance * ndl);

    // Dither before quantising so gradients do not turn into flat bands.
    let noise = hash21(floor(input.clip_position.xy / 4.0)) - 0.5;
    let dithered = lit + vec3<f32>(noise * 0.16);

    return vec4<f32>(posterize(dithered, 5.0), base.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return pixel_shade(input, input.color);
}

@fragment
fn fs_main_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = floor(input.uv * 24.0) / 24.0;
    let texel = textureSample(albedo, albedo_sampler, uv);
    return pixel_shade(input, texel * input.color);
}
