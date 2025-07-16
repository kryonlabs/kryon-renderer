// Advanced gradient shader for WebGPU

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_position: vec2<f32>,
}

struct Uniforms {
    view_proj: mat4x4<f32>,
    gradient_start: vec4<f32>,
    gradient_end: vec4<f32>,
    gradient_params: vec4<f32>, // x: type (0=linear, 1=radial), y: angle, z: radius, w: aspect
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = uniforms.view_proj * vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.world_position = input.position;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let gradient_type = uniforms.gradient_params.x;
    let angle = uniforms.gradient_params.y;
    let radius = uniforms.gradient_params.z;
    let aspect = uniforms.gradient_params.w;
    
    var t: f32;
    
    if (gradient_type < 0.5) {
        // Linear gradient
        let cos_angle = cos(angle);
        let sin_angle = sin(angle);
        let rotated_uv = vec2<f32>(
            input.uv.x * cos_angle - input.uv.y * sin_angle,
            input.uv.x * sin_angle + input.uv.y * cos_angle
        );
        t = clamp(rotated_uv.x, 0.0, 1.0);
    } else {
        // Radial gradient
        let center = vec2<f32>(0.5, 0.5);
        let adjusted_uv = vec2<f32>(
            (input.uv.x - center.x) * aspect,
            input.uv.y - center.y
        );
        let distance = length(adjusted_uv);
        t = clamp(distance / radius, 0.0, 1.0);
    }
    
    // Smooth interpolation between colors
    let color = mix(uniforms.gradient_start, uniforms.gradient_end, smoothstep(0.0, 1.0, t));
    
    return color;
}