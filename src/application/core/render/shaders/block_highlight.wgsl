struct CameraUniform {
    view_projection: mat4x4<f32>,
};

struct HighlightUniform {
    block_origin: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> highlight: HighlightUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

@vertex
fn vertex_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    let center = vec3<f32>(0.5);
    let expanded = center + (input.position - center) * 1.004;
    let world_position = highlight.block_origin.xyz + expanded;
    return camera.view_projection * vec4<f32>(world_position, 1.0);
}

@fragment
fn fragment_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.01, 0.01, 0.01, 1.0);
}
