use glam::{IVec3, Vec3};

pub(super) const CAMERA_EYE_HEIGHT: f32 = 1.8;

pub(super) fn spawn_position(surface: IVec3) -> Vec3 {
    Vec3::new(surface.x as f32, surface.y as f32 + 1.0, surface.z as f32)
}
