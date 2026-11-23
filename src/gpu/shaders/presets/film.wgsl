// Film: sepia tones, film grain, a faint scratch and darkened corners.

fn film_shade(input: VertexOutput, base: vec4<f32>) -> vec4<f32> {
    let key = key_light(input.world_position);
    let ndl = max(dot(normalize(input.normal), key.direction), 0.0);
    let lit = base.rgb * (ambient_light() + key.radiance * ndl);

    let sepia = vec3<f32>(
        dot(lit, vec3<f32>(0.393, 0.769, 0.189)),
        dot(lit, vec3<f32>(0.349, 0.686, 0.168)),
        dot(lit, vec3<f32>(0.272, 0.534, 0.131)),
    );

    let grain = vec3<f32>((hash21(input.clip_position.xy) - 0.5) * 0.12);
    let scratch = select(1.0, 1.25,
        hash21(vec2<f32>(floor(input.clip_position.x), 0.0)) > 0.995);
    let vignette = 1.0 - 0.40 * smoothstep(0.25, 0.75,
        length(input.uv - vec2<f32>(0.5)));

    return vec4<f32>((sepia + grain) * vignette * scratch, base.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return film_shade(input, input.color);
}

@fragment
fn fs_main_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo, albedo_sampler, input.uv);
    return film_shade(input, texel * input.color);
}
