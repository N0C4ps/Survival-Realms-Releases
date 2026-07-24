use glam::Vec3;

use crate::application::core::blocks::BlockId;

#[derive(Clone, Copy, Debug)]
pub(super) struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub size: f32,
    pub block: BlockId,
    pub age: f32,
    pub lifetime: f32,
    pub rotation: f32,
    pub angular_velocity: f32,
    pub skylight: u8,
}

impl Particle {
    pub fn life_remaining(self) -> f32 {
        (1.0 - self.age / self.lifetime).clamp(0.0, 1.0)
    }
}
