struct PauseEffect {
    texel_size: vec2<f32>,
    blur_enabled: f32,
    darkness: f32,
};

@group(0) @binding(0)
var scene_texture: texture_2d<f32>;

@group(0) @binding(1)
var scene_sampler: sampler;

@group(0) @binding(2)
var<uniform> effect: PauseEffect;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[vertex_index];
    var output: VertexOutput;
    output.clip_position = vec4<f32>(position, 0.0, 1.0);
    output.uv = vec2<f32>((position.x + 1.0) * 0.5, (1.0 - position.y) * 0.5);
    return output;
}

fn scene(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(scene_texture, scene_sampler, uv).rgb;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = scene(input.uv) * 0.20;
    let step = effect.texel_size * 2.0 * effect.blur_enabled;

    color += scene(input.uv + vec2<f32>( step.x, 0.0)) * 0.10;
    color += scene(input.uv + vec2<f32>(-step.x, 0.0)) * 0.10;
    color += scene(input.uv + vec2<f32>(0.0,  step.y)) * 0.10;
    color += scene(input.uv + vec2<f32>(0.0, -step.y)) * 0.10;

    color += scene(input.uv + vec2<f32>( step.x,  step.y)) * 0.06;
    color += scene(input.uv + vec2<f32>(-step.x,  step.y)) * 0.06;
    color += scene(input.uv + vec2<f32>( step.x, -step.y)) * 0.06;
    color += scene(input.uv + vec2<f32>(-step.x, -step.y)) * 0.06;

    color += scene(input.uv + vec2<f32>( step.x * 2.0, 0.0)) * 0.04;
    color += scene(input.uv + vec2<f32>(-step.x * 2.0, 0.0)) * 0.04;
    color += scene(input.uv + vec2<f32>(0.0,  step.y * 2.0)) * 0.04;
    color += scene(input.uv + vec2<f32>(0.0, -step.y * 2.0)) * 0.04;

    return vec4<f32>(color * (1.0 - effect.darkness), 1.0);
}
