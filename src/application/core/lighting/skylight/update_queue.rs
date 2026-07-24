use std::collections::VecDeque;

use crate::application::core::{
    blocks::BlockRegistry,
    world::{BlockChange, World, WorldGenerator},
};

use super::incremental;

#[derive(Default)]
pub struct SkylightUpdateQueue {
    pending: VecDeque<BlockChange>,
}

impl SkylightUpdateQueue {
    pub(crate) fn schedule(&mut self, change: BlockChange) {
        self.pending.push_back(change);
    }

    pub fn process(
        &mut self,
        world: &mut World,
        registry: &BlockRegistry,
        generator: WorldGenerator,
        budget: usize,
    ) -> usize {
        let mut processed = 0;
        while processed < budget {
            let Some(change) = self.pending.pop_front() else {
                break;
            };
            incremental::apply_block_change(world, registry, generator, change);
            processed += 1;
        }
        processed
    }
}
