use std::collections::VecDeque;

use glam::IVec3;
use rustc_hash::FxHashSet;

use crate::application::core::{
    blocks::BlockRegistry,
    world::{ChunkPos, World, split_global},
};

use super::LightLevel;

const DIRECTIONS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

pub(super) fn spread(
    world: &mut World,
    registry: &BlockRegistry,
    mut queue: VecDeque<IVec3>,
) -> FxHashSet<ChunkPos> {
    let mut queued: FxHashSet<_> = queue.iter().copied().collect();
    spread_budgeted(world, registry, &mut queue, &mut queued, usize::MAX)
}

pub(super) fn spread_budgeted(
    world: &mut World,
    registry: &BlockRegistry,
    queue: &mut VecDeque<IVec3>,
    queued: &mut FxHashSet<IVec3>,
    budget: usize,
) -> FxHashSet<ChunkPos> {
    let mut changed_chunks = FxHashSet::default();
    let mut processed = 0;
    while processed < budget {
        let Some(position) = queue.pop_front() else {
            break;
        };
        queued.remove(&position);
        processed += 1;
        if registry.get(world.block(position)).properties().is_opaque() {
            continue;
        }

        let source = LightLevel::new(world.skylight(position));
        let propagated = source.attenuated();
        if propagated == LightLevel::DARK {
            continue;
        }

        for direction in DIRECTIONS {
            let neighbour = position + direction;
            if registry
                .get(world.block(neighbour))
                .properties()
                .is_opaque()
                || world.skylight(neighbour) >= propagated.value()
            {
                continue;
            }

            if world.set_skylight(neighbour, propagated.value()).is_some() {
                changed_chunks.insert(split_global(neighbour).0);
                if queued.insert(neighbour) {
                    queue.push_back(neighbour);
                }
            }
        }
    }
    changed_chunks
}
