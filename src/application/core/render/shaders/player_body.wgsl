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

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(input.position, 1.0);
    output.world_position = input.position;
    output.normal = input.normal;
    output.color = input.color;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sunlight = normalize(vec3<f32>(-0.45, 0.82, -0.35));
    let diffuse = max(dot(normalize(input.normal), sunlight), 0.0);
    let lighting = 0.30 + diffuse * 0.70;
    var color = input.color * lighting;

    let distance = length(input.world_position - camera.world_position.xyz);
    let fog_start = camera.fog_distance.x;
    let fog_end = max(camera.fog_distance.y, fog_start + 0.001);
    let fog = smoothstep(fog_start, fog_end, distance);
    color = mix(color, camera.fog_color.rgb, fog);
    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.0 / camera.visual_settings.x));
    return vec4<f32>(color, 1.0);
}
