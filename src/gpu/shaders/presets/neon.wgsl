// Neon: a dark tube core with a saturated fresnel halo and energy bands.

fn neon_shade(input: VertexOutput, base: vec4<f32>) -> vec4<f32> {
    let normal = normalize(input.normal);
    let view = view_vector(input.world_position);
    let key = key_light(input.world_position);
    let ndl = max(dot(normal, key.direction), 0.0);

    let fresnel = pow(1.0 - max(dot(normal, view), 0.0), 2.0);
    let core = posterize(base.rgb * (0.12 + ndl * 0.35), 3.0);
    let halo = base.rgb * key.radiance * fresnel * 2.2;

    // Bright rings along the local height make a surface read as a light tube.
    let rings = pow(0.5 + 0.5 * sin(input.world_position.y * 18.0), 8.0);
    let sparks = base.rgb * rings * 0.35;

    return vec4<f32>(core + halo + sparks, base.a);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return neon_shade(input, input.color);
}

@fragment
fn fs_main_textured(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(albedo, albedo_sampler, input.uv);
    return neon_shade(input, texel * input.color);
}
