// Vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) page: i32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) page: i32,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.page = model.page;
    return out;
}

// Fragment shader

struct Uniforms {
    color: vec4<f32>,
    page: i32,
    buffer: f32,
    gamma: f32
}

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var samp: sampler;
@group(1) @binding(0)
var<uniform> params: Uniforms;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = textureSample(texture, samp, in.tex_coords)[0];
    let alpha = smoothstep(params.buffer - params.gamma, params.buffer + params.gamma, dist);
    return vec4(params.color.rgb, alpha) * params.color.a;
}
