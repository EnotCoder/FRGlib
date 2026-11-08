// 2D sprites, shapes and text.
//
// Every vertex carries a `space` flag: 1.0 means the position is already in
// screen pixels (text, interface widgets), 0.0 means world units and goes
// through the camera. Everything ends up in the same clip space, so a frame
// can mix the two without a second pipeline.

struct View {
    // xy: centre of the camera in world units.
    camera: vec4<f32>,
    // xy: framebuffer size in pixels.
    screen: vec4<f32>,
    // x: zoom. The other components are padding.
    params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(1) @binding(1)
var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) space: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var pixels = in.position;

    if (in.space < 0.5) {
        let zoom = max(view.params.x, 0.000001);
        pixels = (pixels - view.camera.xy) * zoom + view.screen.xy * 0.5;
    }

    // Screen pixels grow down, clip space grows up.
    let ndc = vec2<f32>(
        pixels.x * 2.0 / view.screen.x - 1.0,
        1.0 - pixels.y * 2.0 / view.screen.y,
    );

    var out: VertexOutput;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Solid shapes use a 1x1 white texture, glyphs use coverage in alpha, and
    // sprites use their own colour. One multiply covers all three.
    let texel = textureSample(sprite_texture, sprite_sampler, in.uv);
    return texel * in.color;
}
