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
var particle_textures: texture_2d_array<f32>;

@group(1) @binding(1)
var particle_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) size: f32,
    @location(2) rotation: f32,
    @location(3) texture_layer: u32,
    @location(4) skylight: u32,
    @location(5) opacity: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) texture_layer: u32,
    @location(2) @interpolate(flat) skylight: u32,
    @location(3) @interpolate(flat) opacity: f32,
};

@vertex
fn vertex_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>( 0.5,  0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5,  0.5),
        vec2<f32>(-0.5,  0.5),
    );
    let uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );
    let unrotated = corners[vertex_index] * input.size;
    let sine = sin(input.rotation);
    let cosine = cos(input.rotation);
    let corner = vec2<f32>(
        unrotated.x * cosine - unrotated.y * sine,
        unrotated.x * sine + unrotated.y * cosine,
    );
    let world_position = input.position
        + camera.camera_right.xyz * corner.x
        + camera.camera_up.xyz * corner.y;
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4<f32>(world_position, 1.0);
    output.uv = uvs[vertex_index];
    output.texture_layer = input.texture_layer;
    output.skylight = input.skylight;
    output.opacity = input.opacity;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(
        particle_textures,
        particle_sampler,
        input.uv,
        i32(input.texture_layer),
    );
    if (color.a < 0.05) {
        discard;
    }
    let normalized_skylight = f32(input.skylight) / 15.0;
    let skylight = 0.025 + 0.975 * pow(normalized_skylight, 1.35);
    let lit_color = color.rgb * skylight;
    let tinted_color = select(
        lit_color,
        mix(lit_color, camera.fog_color.rgb, camera.fog_color.a),
        camera.fog_color.a > 0.0,
    );
    let gamma_corrected = pow(max(tinted_color, vec3<f32>(0.0)), vec3<f32>(1.0 / camera.visual_settings.x));
    return vec4<f32>(gamma_corrected, color.a * input.opacity);
}
