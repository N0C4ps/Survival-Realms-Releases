@group(0) @binding(0)
var block_textures: texture_2d_array<f32>;

@group(0) @binding(1)
var block_sampler: sampler;

@group(1) @binding(0)
var ui_textures: texture_2d_array<f32>;

@group(1) @binding(1)
var ui_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) texture_kind: u32,
    @location(3) texture_layer: u32,
    @location(4) shade: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) texture_kind: u32,
    @location(2) @interpolate(flat) texture_layer: u32,
    @location(3) shade: f32,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.texture_kind = input.texture_kind;
    output.texture_layer = input.texture_layer;
    output.shade = input.shade;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec4<f32>;
    if (input.texture_kind == 0u) {
        color = textureSample(
            block_textures,
            block_sampler,
            input.uv,
            i32(input.texture_layer),
        );
    } else {
        color = textureSample(
            ui_textures,
            ui_sampler,
            input.uv,
            i32(input.texture_layer),
        );
    }
    if (color.a < 0.02) {
        discard;
    }
    return vec4<f32>(color.rgb * input.shade, color.a);
}
