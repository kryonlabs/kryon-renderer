// Advanced text rendering shader for WebGPU

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct Uniforms {
    view_proj: mat4x4<f32>,
    text_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var atlas_texture: texture_2d<f32>;

@group(0) @binding(2)
var atlas_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_texture, atlas_sampler, input.tex_coords).r;
    
    // Apply text color with alpha from texture
    var color = uniforms.text_color;
    color.a *= alpha;
    
    // Discard transparent pixels
    if (color.a < 0.01) {
        discard;
    }
    
    return color;
}