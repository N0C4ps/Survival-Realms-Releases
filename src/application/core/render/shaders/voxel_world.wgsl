struct Camera {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    world_position: vec4<f32>,
    fog_distance: vec4<f32>,
    fog_color: vec4<f32>,
    visual_settings: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var block_textures: texture_2d_array<f32>;

@group(1) @binding(1)
var block_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) texture_layer: u32,
    @location(4) skylight: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) texture_layer: u32,
    @location(3) @interpolate(flat) skylight: u32,
    @location(4) world_position: vec3<f32>,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(input.position, 1.0);
    output.normal = input.normal;
    output.uv = input.uv;
    output.texture_layer = input.texture_layer;
    output.skylight = input.skylight;
    output.world_position = input.position;
    return output;
}

const SKY_TOP: vec3<f32> = vec3<f32>(0.004025, 0.184475, 0.991102);
const SKY_PEAK: vec3<f32> = vec3<f32>(0.198069, 0.806952, 1.0);
const SKY_HORIZON: vec3<f32> = vec3<f32>(0.107023, 0.617207, 0.991102);
const SKY_BELOW: vec3<f32> = vec3<f32>(0.001821, 0.068478, 0.304987);

fn smootherstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let amount = clamp((value - edge_start) / (edge_end - edge_start), 0.0, 1.0);
    return amount * amount * amount * (amount * (amount * 6.0 - 15.0) + 10.0);
}

fn sky_color(direction: vec3<f32>) -> vec3<f32> {
    if (direction.y >= 0.0) {
        let cyan_arrival = smootherstep(-0.02, 0.30, direction.y);
        let top_arrival = smootherstep(0.42, 1.0, direction.y);
        return mix(mix(SKY_HORIZON, SKY_PEAK, cyan_arrival), SKY_TOP, top_arrival);
    }
    let lower_gradient = smootherstep(-1.0, 0.05, direction.y);
    return mix(SKY_BELOW, SKY_HORIZON, lower_gradient);
}

fn shade_voxel(input: VertexOutput) -> vec4<f32> {
    let texture_color = textureSample(
        block_textures,
        block_sampler,
        input.uv,
        i32(input.texture_layer),
    );
    let sun_direction = normalize(vec3<f32>(0.4, 1.0, 0.25));
    let directional_light = 0.48 + 0.52 * max(dot(input.normal, sun_direction), 0.0);
    let normalized_skylight = f32(input.skylight) / 15.0;
    let skylight = 0.025 + 0.975 * pow(normalized_skylight, 1.35);
    let raw_lit_color = texture_color.rgb * directional_light * skylight;
    let lit_color = mix(
        raw_lit_color,
        camera.fog_color.rgb,
        camera.fog_color.a,
    );

    let camera_offset = input.world_position - camera.world_position.xyz;
    let distance_from_camera = length(camera_offset);
    let fog_amount = smootherstep(
        camera.fog_distance.x,
        camera.fog_distance.y,
        distance_from_camera,
    );
    let sky_fog = sky_color(normalize(camera_offset));
    let fog_color = select(sky_fog, camera.fog_color.rgb, camera.fog_color.a > 0.0);
    let fogged_color = mix(lit_color, fog_color, fog_amount);
    let gamma_corrected = pow(max(fogged_color, vec3<f32>(0.0)), vec3<f32>(1.0 / camera.visual_settings.x));
    return vec4<f32>(gamma_corrected, texture_color.a);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = shade_voxel(input);
    if (shaded.a < 0.5) {
        discard;
    }
    return shaded;
}

@fragment
fn liquid_fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = shade_voxel(input);
    let water_alpha = 0.68;
    let block_id = input.texture_layer / 3u;
    let alpha = select(1.0, water_alpha, block_id == 6u);
    return vec4<f32>(shaded.rgb, alpha);
}
