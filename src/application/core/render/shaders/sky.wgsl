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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

const SKY_TOP: vec3<f32> = vec3<f32>(0.004025, 0.184475, 0.991102);
const SKY_PEAK: vec3<f32> = vec3<f32>(0.198069, 0.806952, 1.0);
const SKY_HORIZON: vec3<f32> = vec3<f32>(0.107023, 0.617207, 0.991102);
const SKY_BELOW: vec3<f32> = vec3<f32>(0.001821, 0.068478, 0.304987);

fn smootherstep(edge_start: f32, edge_end: f32, value: f32) -> f32 {
    let amount = clamp((value - edge_start) / (edge_end - edge_start), 0.0, 1.0);
    return amount * amount * amount * (amount * (amount * 6.0 - 15.0) + 10.0);
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: VertexOutput;
    output.ndc = positions[vertex_index];
    output.clip_position = vec4<f32>(output.ndc, 1.0, 1.0);
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if (camera.fog_color.a > 0.0) {
        let submerged = pow(camera.fog_color.rgb, vec3<f32>(1.0 / camera.visual_settings.x));
        return vec4<f32>(submerged, 1.0);
    }
    let far_point = camera.inverse_view_projection * vec4<f32>(input.ndc, 1.0, 1.0);
    let world_point = far_point.xyz / far_point.w;
    let direction = normalize(world_point - camera.world_position.xyz);

    var color: vec3<f32>;
    if (direction.y >= 0.0) {
        let cyan_arrival = smootherstep(-0.02, 0.30, direction.y);
        let top_arrival = smootherstep(0.42, 1.0, direction.y);
        let broad_cyan = mix(SKY_HORIZON, SKY_PEAK, cyan_arrival);
        color = mix(broad_cyan, SKY_TOP, top_arrival);
    } else {
        let lower_gradient = smootherstep(-1.0, 0.05, direction.y);
        color = mix(SKY_BELOW, SKY_HORIZON, lower_gradient);
    }

    let sun_direction = normalize(vec3<f32>(0.35, 0.68, -0.48));
    let looking_at_sun = max(dot(direction, sun_direction), 0.0);
    let sun_disc = smoothstep(0.9985, 0.9996, looking_at_sun);
    let sun_glow = pow(looking_at_sun, 72.0) * 0.18;
    color = color + vec3<f32>(1.0, 0.63, 0.20) * sun_glow;
    color = mix(color, vec3<f32>(1.0, 0.88, 0.52), sun_disc);

    let gamma_corrected = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / camera.visual_settings.x));
    return vec4<f32>(gamma_corrected, 1.0);
}
