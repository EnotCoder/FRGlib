// Toon: hard light bands, a single inked highlight and a dark silhouette rim.

fn toon_shade(
    world_position: vec3<f32>,
    normal_in: vec3<f32>,
    base: vec4<f32>,
) -> vec4<f32> {
    let normal = normalize(normal_in);
    let view = view_vector(world_position);
    let key = key_light(world_position);
    let ndl = max(dot(normal, key.direction), 0.0);

    // Three or four flat steps instead of a smooth diffuse gradient.
    let band = floor(ndl * 3.0 + 0.35) / 3.0;
    var color = base.rgb * (ambient_light() + key.radiance * band);

    // Hand-drawn style: the specular is either fully on or fully off.
    let half_vector = normalize(key.direction + view);
    let highlight = pow(max(dot(normal, half_vector), 0.0), 48.0);
    if (highlight > 0.35) {
        color += vec3<f32>(0.35);
    }

    // A dark rim fakes an outline without a second geometry pass.
    let rim = 1.0 - max(dot(normal, view), 0.0);
    if (rim > 0.72) {
        color *= 0.22;
    }
    return vec4<f32>(color, base.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return toon_shade(input.world_position, input.normal, input.color);
}

@fragment
fn fs_main_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo, albedo_sampler, input.uv);
    return toon_shade(input.world_position, input.normal, texel * input.color);
}
