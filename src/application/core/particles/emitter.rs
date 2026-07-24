use glam::{IVec3, Vec3};

use crate::application::core::blocks::BlockId;

use super::particle::Particle;

pub(super) const PARTICLES_PER_BLOCK: usize = 32;

pub(super) fn block_break(
    particles: &mut Vec<Particle>,
    random_state: &mut u64,
    position: IVec3,
    block: BlockId,
) {
    let center = position.as_vec3() + Vec3::splat(0.5);
    for _ in 0..PARTICLES_PER_BLOCK {
        let offset = Vec3::new(
            signed_random(random_state) * 0.44,
            signed_random(random_state) * 0.44,
            signed_random(random_state) * 0.44,
        );
        let direction = offset.normalize_or_zero();
        let radial_speed = 0.65 + unit_random(random_state) * 0.85;
        let velocity = direction * radial_speed
            + Vec3::new(
                signed_random(random_state) * 0.18,
                0.65 + unit_random(random_state) * 0.55,
                signed_random(random_state) * 0.18,
            );
        particles.push(Particle {
            position: center + offset,
            velocity,
            size: 0.20 + unit_random(random_state) * 0.10,
            block,
            age: 0.0,
            lifetime: 1.15 + unit_random(random_state) * 0.65,
            rotation: unit_random(random_state) * std::f32::consts::TAU,
            angular_velocity: signed_random(random_state) * 5.0,
            skylight: 15,
        });
    }
}

fn unit_random(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state as u32) as f32 / u32::MAX as f32
}

fn signed_random(state: &mut u64) -> f32 {
    unit_random(state) * 2.0 - 1.0
}
