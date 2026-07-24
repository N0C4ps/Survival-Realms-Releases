use std::collections::VecDeque;

use glam::{IVec3, UVec3};
use rustc_hash::FxHashSet;

use crate::application::core::{
    blocks::BlockRegistry,
    world::{CHUNK_SIZE, ChunkPos, World, split_global},
};

use super::propagation;

const DIRECTIONS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

#[derive(Default)]
pub struct SkylightPropagationQueue {
    pending: VecDeque<IVec3>,
    queued: FxHashSet<IVec3>,
}

impl SkylightPropagationQueue {
    pub fn schedule_chunk(
        &mut self,
        world: &World,
        registry: &BlockRegistry,
        chunk_position: ChunkPos,
    ) {
        self.schedule_whole_chunk(world, registry, chunk_position);
        for direction in DIRECTIONS {
            self.schedule_neighbour_border(world, registry, chunk_position + direction, -direction);
        }
    }

    pub fn process(&mut self, world: &mut World, registry: &BlockRegistry, budget: usize) -> usize {
        let changed = propagation::spread_budgeted(
            world,
            registry,
            &mut self.pending,
            &mut self.queued,
            budget,
        );
        let changed_count = changed.len();
        world.mark_lighting_chunks_dirty(&changed);
        changed_count
    }

    fn schedule_whole_chunk(
        &mut self,
        world: &World,
        registry: &BlockRegistry,
        chunk_position: ChunkPos,
    ) {
        let Some(chunk) = world.chunk(chunk_position) else {
            return;
        };
        let origin = chunk_position * CHUNK_SIZE as i32;
        for x in 0..CHUNK_SIZE as u32 {
            for y in 0..CHUNK_SIZE as u32 {
                for z in 0..CHUNK_SIZE as u32 {
                    let local = UVec3::new(x, y, z);
                    if chunk.skylight(local) == 0 {
                        continue;
                    }
                    self.schedule_if_source(world, registry, origin + local.as_ivec3());
                }
            }
        }
    }

    fn schedule_neighbour_border(
        &mut self,
        world: &World,
        registry: &BlockRegistry,
        chunk_position: ChunkPos,
        border_direction: IVec3,
    ) {
        if world.chunk(chunk_position).is_none() {
            return;
        }
        let maximum = (CHUNK_SIZE - 1) as u32;
        for a in 0..CHUNK_SIZE as u32 {
            for b in 0..CHUNK_SIZE as u32 {
                let local = if border_direction == IVec3::X {
                    UVec3::new(maximum, a, b)
                } else if border_direction == IVec3::NEG_X {
                    UVec3::new(0, a, b)
                } else if border_direction == IVec3::Y {
                    UVec3::new(a, maximum, b)
                } else if border_direction == IVec3::NEG_Y {
                    UVec3::new(a, 0, b)
                } else if border_direction == IVec3::Z {
                    UVec3::new(a, b, maximum)
                } else {
                    UVec3::new(a, b, 0)
                };
                let global = chunk_position * CHUNK_SIZE as i32 + local.as_ivec3();
                self.schedule_if_source(world, registry, global);
            }
        }
    }

    fn schedule_if_source(&mut self, world: &World, registry: &BlockRegistry, position: IVec3) {
        let level = world.skylight(position);
        if level <= 1 || registry.get(world.block(position)).properties().is_opaque() {
            return;
        }

        let can_spread = DIRECTIONS.into_iter().any(|direction| {
            let neighbour = position + direction;
            let neighbour_chunk = split_global(neighbour).0;
            world.chunk(neighbour_chunk).is_some()
                && !registry
                    .get(world.block(neighbour))
                    .properties()
                    .is_opaque()
                && world.skylight(neighbour) + 1 < level
        });
        if can_spread && self.queued.insert(position) {
            self.pending.push_back(position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::core::blocks::BlockRegistry;

    #[test]
    fn loaded_ravine_light_spreads_smoothly_across_chunk_borders() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Default::default());
        world.insert_chunk(IVec3::X, Default::default());
        let source = IVec3::new(CHUNK_SIZE as i32 - 1, 8, 8);
        world.set_skylight(source, 15);

        let mut propagation = SkylightPropagationQueue::default();
        propagation.schedule_chunk(&world, &registry, IVec3::ZERO);
        propagation.process(&mut world, &registry, usize::MAX);

        assert_eq!(world.skylight(source), 15);
        assert_eq!(world.skylight(source + IVec3::X), 14);
        assert_eq!(world.skylight(source + IVec3::X * 14), 1);
        assert_eq!(world.skylight(source + IVec3::X * 15), 0);
    }
}
