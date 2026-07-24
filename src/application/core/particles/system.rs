use std::time::Duration;

use glam::{IVec3, Vec3};

use crate::application::core::{
    blocks::{BlockId, BlockRegistry},
    world::World,
};

use super::{assets, emitter, particle::Particle};

const MAX_PARTICLES: usize = 2_048;
const GRAVITY: f32 = -22.0;

pub(crate) struct ParticleSystem {
    particles: Vec<Particle>,
    random_state: u64,
}

impl ParticleSystem {
    pub fn emit_block_break(&mut self, position: IVec3, block: BlockId) {
        if assets::texture_path(block).is_none() {
            return;
        }
        let overflow = self
            .particles
            .len()
            .saturating_add(emitter::PARTICLES_PER_BLOCK)
            .saturating_sub(MAX_PARTICLES);
        if overflow > 0 {
            self.particles.drain(..overflow);
        }
        emitter::block_break(&mut self.particles, &mut self.random_state, position, block);
    }

    pub fn update(&mut self, delta_time: Duration, world: &World, registry: &BlockRegistry) {
        let delta = delta_time.as_secs_f32().min(0.05);
        self.particles.retain_mut(|particle| {
            particle.age += delta;
            particle.velocity.y += GRAVITY * delta;
            particle.velocity *= 1.0 / (1.0 + 0.7 * delta);
            particle.rotation += particle.angular_velocity * delta;
            let next = particle.position + particle.velocity * delta;
            if particle.age >= particle.lifetime
                || touches_ground(next, particle.size, world, registry)
            {
                return false;
            }
            particle.position = next;
            particle.skylight = world.skylight(next.floor().as_ivec3());
            true
        });
    }

    pub(super) fn particles(&self) -> &[Particle] {
        &self.particles
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.particles.len()
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self {
            particles: Vec::new(),
            random_state: 0xD1B5_4A32_D192_ED03,
        }
    }
}

fn touches_ground(position: Vec3, size: f32, world: &World, registry: &BlockRegistry) -> bool {
    let feet = position - Vec3::Y * size * 0.5;
    let block_position = feet.floor().as_ivec3();
    registry
        .get(world.block(block_position))
        .properties()
        .is_solid()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaking_a_block_emits_multiple_matching_particles() {
        let mut system = ParticleSystem::default();
        system.emit_block_break(IVec3::ZERO, BlockId::GRASS);

        assert_eq!(system.len(), emitter::PARTICLES_PER_BLOCK);
        assert!(
            system
                .particles()
                .iter()
                .all(|particle| particle.block == BlockId::GRASS)
        );
    }

    #[test]
    fn particles_disappear_when_they_reach_a_solid_floor() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.set_block(IVec3::ZERO, BlockId::STONE);
        let mut system = ParticleSystem::default();
        system.particles.push(Particle {
            position: Vec3::new(0.5, 1.05, 0.5),
            velocity: Vec3::new(0.0, -2.0, 0.0),
            size: 0.2,
            block: BlockId::STONE,
            age: 0.0,
            lifetime: 2.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            skylight: 15,
        });

        system.update(Duration::from_millis(16), &world, &registry);

        assert_eq!(system.len(), 0);
    }

    #[test]
    fn particles_sample_the_worlds_sixteen_skylight_levels() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.set_block(IVec3::new(0, -1, 0), BlockId::STONE);
        world.set_block(IVec3::ZERO, BlockId::AIR);
        world.set_skylight(IVec3::ZERO, 6);
        let mut system = ParticleSystem::default();
        system.particles.push(Particle {
            position: Vec3::new(0.5, 0.5, 0.5),
            velocity: Vec3::ZERO,
            size: 0.2,
            block: BlockId::STONE,
            age: 0.0,
            lifetime: 2.0,
            rotation: 0.0,
            angular_velocity: 0.0,
            skylight: 15,
        });

        system.update(Duration::from_millis(1), &world, &registry);

        assert_eq!(system.particles()[0].skylight, 6);
    }
}
