use glam::{Mat4, Vec3};

use super::{direction, projection::Projection};

pub struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    projection: Projection,
}

impl Camera {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,
            projection: Projection::new(aspect_ratio),
        }
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection.matrix() * Mat4::look_to_rh(self.position, self.forward(), Vec3::Y)
    }

    pub fn forward(&self) -> Vec3 {
        direction::forward(self.yaw, self.pitch)
    }

    pub fn right(&self) -> Vec3 {
        self.horizontal_forward().cross(Vec3::Y).normalize()
    }

    pub(crate) fn horizontal_forward(&self) -> Vec3 {
        let forward = self.forward();
        Vec3::new(forward.x, 0.0, forward.z).normalize()
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub(crate) fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub(super) fn rotate(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.yaw += yaw_delta;
        self.pitch = (self.pitch + pitch_delta)
            .clamp(-direction::MAX_PITCH_RADIANS, direction::MAX_PITCH_RADIANS);
    }

    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.projection.set_aspect_ratio(aspect_ratio);
    }

    pub(crate) fn set_fov_degrees(&mut self, degrees: f32) {
        self.projection.set_fov_degrees(degrees);
    }
}
