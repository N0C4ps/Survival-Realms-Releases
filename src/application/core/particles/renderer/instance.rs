use bytemuck::{Pod, Zeroable};

use super::super::particle::Particle;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct ParticleInstance {
    position: [f32; 3],
    size: f32,
    rotation: f32,
    texture_layer: u32,
    skylight: u32,
    opacity: f32,
}

impl From<&Particle> for ParticleInstance {
    fn from(particle: &Particle) -> Self {
        Self {
            position: particle.position.to_array(),
            size: particle.size * (0.35 + 0.65 * particle.life_remaining()),
            rotation: particle.rotation,
            texture_layer: u32::from(particle.block.value()),
            skylight: u32::from(particle.skylight),
            opacity: particle.life_remaining() * 0.9,
        }
    }
}
