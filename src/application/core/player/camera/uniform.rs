use bytemuck::{Pod, Zeroable};

use super::state::Camera;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct CameraUniform {
    view_projection: [[f32; 4]; 4],
    inverse_view_projection: [[f32; 4]; 4],
    world_position: [f32; 4],
    fog_distance: [f32; 4],
    fog_color: [f32; 4],
    visual_settings: [f32; 4],
    camera_right: [f32; 4],
    camera_up: [f32; 4],
}

impl CameraUniform {
    pub fn from_camera(
        camera: &Camera,
        fog_distance: [f32; 4],
        fog_color: [f32; 4],
        gamma: f32,
    ) -> Self {
        let view_projection = camera.view_projection_matrix();
        let position = camera.position();
        let right = camera.right();
        let up = right.cross(camera.forward()).normalize();

        Self {
            view_projection: view_projection.to_cols_array_2d(),
            inverse_view_projection: view_projection.inverse().to_cols_array_2d(),
            world_position: [position.x, position.y, position.z, 1.0],
            fog_distance,
            fog_color,
            visual_settings: [gamma, 0.0, 0.0, 0.0],
            camera_right: [right.x, right.y, right.z, 0.0],
            camera_up: [up.x, up.y, up.z, 0.0],
        }
    }
}
