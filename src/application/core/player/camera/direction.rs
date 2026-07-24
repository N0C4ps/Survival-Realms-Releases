use glam::Vec3;

pub(super) const MAX_PITCH_RADIANS: f32 = 89.0_f32.to_radians();

pub(super) fn forward(yaw: f32, pitch: f32) -> Vec3 {
    Vec3::new(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    )
    .normalize()
}
