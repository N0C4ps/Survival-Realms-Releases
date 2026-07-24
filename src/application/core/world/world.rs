use std::collections::VecDeque;

use glam::{IVec3, UVec3};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::application::core::blocks::BlockId;

use super::{
    BlockChange, FluidState,
    chunk::{CHUNK_SIZE, Chunk, ChunkFlags, ChunkPos},
    coordinates,
};

#[derive(Default)]
pub struct World {
    chunks: FxHashMap<ChunkPos, Chunk>,
    edits: FxHashMap<ChunkPos, FxHashMap<UVec3, BlockId>>,
    fluid_states: FxHashMap<IVec3, FluidState>,
    pending_light_updates: VecDeque<BlockChange>,
    pending_fluid_updates: VecDeque<BlockChange>,
    pending_mesh_priorities: VecDeque<ChunkPos>,
    queued_mesh_priorities: FxHashSet<ChunkPos>,
    pending_skylight_chunks: VecDeque<ChunkPos>,
    pending_dirty_chunks: VecDeque<ChunkPos>,
    queued_dirty_chunks: FxHashSet<ChunkPos>,
    pending_removed_chunks: VecDeque<ChunkPos>,
    pending_grass_chunks: VecDeque<ChunkPos>,
    pending_fluid_chunks: VecDeque<ChunkPos>,
    pending_physics_chunks: VecDeque<ChunkPos>,
    pending_grass_light_chunks: VecDeque<ChunkPos>,
    queued_grass_light_chunks: FxHashSet<ChunkPos>,
}

impl World {
    pub(crate) fn has_chunk_edits(&self, position: ChunkPos) -> bool {
        self.edits
            .get(&position)
            .is_some_and(|edits| !edits.is_empty())
    }

    pub(crate) fn restore_edit(&mut self, global: IVec3, block: BlockId) {
        let (chunk_position, local) = coordinates::split_global(global);
        self.edits
            .entry(chunk_position)
            .or_default()
            .insert(local, block);
    }

    pub(crate) fn persistent_edits(
        &self,
        generator: super::WorldGenerator,
    ) -> Vec<(IVec3, BlockId)> {
        let mut edits = Vec::new();
        let original = generator.original_block_sampler();
        for (&chunk_position, chunk_edits) in &self.edits {
            for (&local, &block) in chunk_edits {
                let global = coordinates::global_from_local(chunk_position, local);
                if original.block(global) != Some(block) {
                    edits.push((global, block));
                }
            }
        }
        edits
    }

    pub fn chunk(&self, position: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&position)
    }

    pub fn chunk_mut(&mut self, position: ChunkPos) -> Option<&mut Chunk> {
        self.chunks.get_mut(&position)
    }

    pub fn insert_chunk(&mut self, position: ChunkPos, mut chunk: Chunk) -> Option<Chunk> {
        let mut applied_edits = Vec::new();
        if let Some(edits) = self.edits.get(&position) {
            for (&local, &block) in edits {
                let previous = chunk.set_block(local, block);
                if previous != block {
                    applied_edits.push(BlockChange {
                        position: coordinates::global_from_local(position, local),
                        previous,
                        current: block,
                    });
                }
                if block == BlockId::GRASS {
                    chunk.flags_mut().insert(ChunkFlags::GRASS_TICKS);
                }
            }
        }
        if !applied_edits.is_empty() && chunk.flags().contains(ChunkFlags::FLUID_SOURCES) {
            chunk.rebuild_fluid_source_candidates();
        }
        let has_light_sources = chunk.flags().contains(ChunkFlags::LIGHT_SOURCES);
        let previous = self.chunks.insert(position, chunk);
        self.queue_dirty_chunk(position);
        if has_light_sources {
            self.pending_skylight_chunks.push_back(position);
        }
        // Every arriving chunk may expose the underside of a liquid source in
        // the chunk above, even when this chunk contains no liquid itself.
        self.pending_fluid_chunks.push_back(position);
        self.pending_physics_chunks.push_back(position);
        if self
            .chunks
            .get(&position)
            .is_some_and(|chunk| chunk.flags().contains(ChunkFlags::GRASS_TICKS))
        {
            self.pending_grass_chunks.push_back(position);
        }
        for change in applied_edits {
            self.pending_light_updates.push_back(change);
            self.pending_fluid_updates.push_back(change);
            self.queue_mesh_priority(change.position);
        }
        self.mark_neighbours_dirty(position);
        previous
    }

    pub fn remove_chunk(&mut self, position: ChunkPos) -> Option<Chunk> {
        let removed = self.chunks.remove(&position);
        if removed.is_some() {
            self.fluid_states
                .retain(|&global, _| coordinates::split_global(global).0 != position);
            self.pending_removed_chunks.push_back(position);
            self.queued_dirty_chunks.remove(&position);
            self.mark_neighbours_dirty(position);
        }
        removed
    }

    pub fn remove_chunks(&mut self, positions: &[ChunkPos]) -> usize {
        if positions.is_empty() {
            return 0;
        }

        let requested: FxHashSet<_> = positions.iter().copied().collect();
        let mut removed_positions = Vec::with_capacity(positions.len());
        for &position in positions {
            if self.chunks.remove(&position).is_some() {
                removed_positions.push(position);
                self.pending_removed_chunks.push_back(position);
                self.queued_dirty_chunks.remove(&position);
                self.queued_mesh_priorities.remove(&position);
                self.queued_grass_light_chunks.remove(&position);
            }
        }
        if removed_positions.is_empty() {
            return 0;
        }

        self.fluid_states
            .retain(|&global, _| !requested.contains(&coordinates::split_global(global).0));

        let mut neighbours = FxHashSet::default();
        for &position in &removed_positions {
            for direction in [
                IVec3::X,
                IVec3::NEG_X,
                IVec3::Y,
                IVec3::NEG_Y,
                IVec3::Z,
                IVec3::NEG_Z,
            ] {
                let neighbour = position + direction;
                if !requested.contains(&neighbour) {
                    neighbours.insert(neighbour);
                }
            }
        }
        for neighbour in neighbours {
            self.mark_chunk_dirty(neighbour);
        }

        removed_positions.len()
    }

    pub fn chunks(&self) -> impl ExactSizeIterator<Item = (&ChunkPos, &Chunk)> {
        self.chunks.iter()
    }

    pub fn block(&self, global: IVec3) -> BlockId {
        let (chunk_position, local) = coordinates::split_global(global);
        self.chunk(chunk_position)
            .map_or(BlockId::AIR, |chunk| chunk.block(local))
    }

    pub fn skylight(&self, global: IVec3) -> u8 {
        let (chunk_position, local) = coordinates::split_global(global);
        self.chunk(chunk_position)
            .map_or(0, |chunk| chunk.skylight(local))
    }

    pub fn set_skylight(&mut self, global: IVec3, level: u8) -> Option<u8> {
        let (chunk_position, local) = coordinates::split_global(global);
        self.chunk_mut(chunk_position)
            .map(|chunk| chunk.set_skylight(local, level))
    }

    pub fn clear_skylight(&mut self) {
        for chunk in self.chunks.values_mut() {
            chunk.clear_skylight();
        }
    }

    pub fn mark_all_dirty(&mut self) {
        let positions: Vec<_> = self.chunks.keys().copied().collect();
        for position in positions {
            self.mark_chunk_dirty(position);
        }
    }

    pub fn mark_lighting_chunks_dirty(&mut self, changed: &FxHashSet<ChunkPos>) {
        for &position in changed {
            if self.queued_grass_light_chunks.insert(position) {
                self.pending_grass_light_chunks.push_back(position);
            }
            self.mark_chunk_dirty(position);
            for direction in [
                IVec3::X,
                IVec3::NEG_X,
                IVec3::Y,
                IVec3::NEG_Y,
                IVec3::Z,
                IVec3::NEG_Z,
            ] {
                self.mark_chunk_dirty(position + direction);
            }
        }
    }

    pub fn set_block(&mut self, global: IVec3, block: BlockId) -> BlockId {
        let (chunk_position, local) = coordinates::split_global(global);
        let chunk = self.chunks.entry(chunk_position).or_default();
        chunk.flags_mut().insert(ChunkFlags::LOADED);
        let previous = chunk.set_block(local, block);
        if !block.is_liquid() {
            self.fluid_states.remove(&global);
        }
        self.queue_dirty_chunk(chunk_position);
        self.mark_border_neighbours_dirty(chunk_position, local);
        previous
    }

    pub fn edit_block(&mut self, global: IVec3, block: BlockId) -> Option<BlockId> {
        let (chunk_position, local) = coordinates::split_global(global);
        let chunk = self.chunk_mut(chunk_position)?;
        let previous = chunk.set_block(local, block);
        if !block.is_liquid() {
            self.fluid_states.remove(&global);
        }
        if previous == block {
            if block == BlockId::WATER || block == BlockId::LAVA {
                self.edits
                    .entry(chunk_position)
                    .or_default()
                    .insert(local, block);
                self.pending_fluid_updates.push_back(BlockChange {
                    position: global,
                    previous,
                    current: block,
                });
            }
            return Some(previous);
        }
        self.queue_dirty_chunk(chunk_position);

        self.edits
            .entry(chunk_position)
            .or_default()
            .insert(local, block);
        self.pending_light_updates.push_back(BlockChange {
            position: global,
            previous,
            current: block,
        });
        self.pending_fluid_updates.push_back(BlockChange {
            position: global,
            previous,
            current: block,
        });
        self.queue_mesh_priority(global);
        self.mark_border_neighbours_dirty(chunk_position, local);
        Some(previous)
    }

    pub(crate) fn take_pending_light_updates(&mut self) -> Vec<BlockChange> {
        self.pending_light_updates.drain(..).collect()
    }

    pub(crate) fn take_pending_fluid_updates(&mut self) -> Vec<BlockChange> {
        self.pending_fluid_updates.drain(..).collect()
    }

    pub(crate) fn contains_block(&self, global: IVec3) -> bool {
        self.chunks
            .contains_key(&coordinates::split_global(global).0)
    }

    pub(crate) fn fluid_state(&self, global: IVec3) -> Option<FluidState> {
        self.fluid_states.get(&global).copied()
    }

    pub(crate) fn liquid_height(&self, global: IVec3) -> Option<f32> {
        let block = self.block(global);
        if !block.is_liquid() {
            return None;
        }
        let state = self.fluid_state(global).unwrap_or(FluidState::SOURCE);
        if state.is_falling() || self.block(global + IVec3::Y) == block {
            return Some(1.0);
        }
        if state.level() == 0 {
            return Some(0.92);
        }

        let maximum_level: u8 = if block == BlockId::WATER { 8 } else { 5 };
        let level = state.level().min(maximum_level);
        let remaining = maximum_level.saturating_sub(level) as f32;
        Some(0.125 + remaining / (maximum_level - 1) as f32 * 0.75)
    }

    pub(crate) fn set_fluid_state(&mut self, global: IVec3, state: FluidState) -> bool {
        if !self.contains_block(global) || !self.block(global).is_liquid() {
            return false;
        }
        if self.fluid_states.insert(global, state) == Some(state) {
            return false;
        }
        let (chunk_position, local) = coordinates::split_global(global);
        self.mark_chunk_dirty(chunk_position);
        self.mark_border_neighbours_dirty(chunk_position, local);
        self.queue_mesh_priority(global);
        true
    }

    pub(crate) fn clear_fluid_state(&mut self, global: IVec3) -> bool {
        if self.fluid_states.remove(&global).is_none() {
            return false;
        }
        let (chunk_position, local) = coordinates::split_global(global);
        self.mark_chunk_dirty(chunk_position);
        self.mark_border_neighbours_dirty(chunk_position, local);
        self.queue_mesh_priority(global);
        true
    }

    /// Changes a simulated block without recording it as a player/world edit.
    ///
    /// Fluid currents use this path so only their source block is persisted.
    pub(crate) fn set_simulated_block(&mut self, global: IVec3, block: BlockId) -> Option<BlockId> {
        let (chunk_position, local) = coordinates::split_global(global);
        let chunk = self.chunk_mut(chunk_position)?;
        let previous = chunk.set_block(local, block);
        if !block.is_liquid() {
            self.fluid_states.remove(&global);
        }
        if previous == block {
            return Some(previous);
        }

        self.queue_dirty_chunk(chunk_position);
        self.pending_light_updates.push_back(BlockChange {
            position: global,
            previous,
            current: block,
        });
        self.queue_mesh_priority(global);
        self.mark_border_neighbours_dirty(chunk_position, local);
        Some(previous)
    }

    pub(crate) fn take_pending_skylight_chunks(&mut self) -> Vec<ChunkPos> {
        self.pending_skylight_chunks.drain(..).collect()
    }

    pub(crate) fn take_dirty_chunks(&mut self) -> Vec<ChunkPos> {
        self.queued_dirty_chunks.clear();
        self.pending_dirty_chunks.drain(..).collect()
    }

    pub(crate) fn take_removed_chunks(&mut self) -> Vec<ChunkPos> {
        self.pending_removed_chunks.drain(..).collect()
    }

    pub(crate) fn take_pending_grass_chunks(&mut self) -> Vec<ChunkPos> {
        self.pending_grass_chunks.drain(..).collect()
    }

    pub(crate) fn take_pending_fluid_chunks(&mut self) -> Vec<ChunkPos> {
        self.pending_fluid_chunks.drain(..).collect()
    }

    pub(crate) fn take_pending_physics_chunks(&mut self) -> Vec<ChunkPos> {
        self.pending_physics_chunks.drain(..).collect()
    }

    pub(crate) fn take_grass_light_chunks(&mut self) -> Vec<ChunkPos> {
        self.queued_grass_light_chunks.clear();
        self.pending_grass_light_chunks.drain(..).collect()
    }

    pub fn take_mesh_priorities(&mut self) -> Vec<ChunkPos> {
        self.queued_mesh_priorities.clear();
        self.pending_mesh_priorities.drain(..).collect()
    }

    pub fn edited_block_count(&self) -> usize {
        self.edits.values().map(FxHashMap::len).sum()
    }

    fn queue_mesh_priority(&mut self, global: IVec3) {
        for position in [
            global,
            global + IVec3::X,
            global + IVec3::NEG_X,
            global + IVec3::Y,
            global + IVec3::NEG_Y,
            global + IVec3::Z,
            global + IVec3::NEG_Z,
        ] {
            let chunk = coordinates::split_global(position).0;
            if self.chunks.contains_key(&chunk) && self.queued_mesh_priorities.insert(chunk) {
                self.pending_mesh_priorities.push_back(chunk);
            }
        }
    }

    fn mark_border_neighbours_dirty(&mut self, position: ChunkPos, local: UVec3) {
        let max = (CHUNK_SIZE - 1) as u32;
        let neighbours = [
            (local.x == 0).then_some(position - IVec3::X),
            (local.x == max).then_some(position + IVec3::X),
            (local.y == 0).then_some(position - IVec3::Y),
            (local.y == max).then_some(position + IVec3::Y),
            (local.z == 0).then_some(position - IVec3::Z),
            (local.z == max).then_some(position + IVec3::Z),
        ];

        for neighbour in neighbours.into_iter().flatten() {
            self.mark_chunk_dirty(neighbour);
        }
    }

    fn mark_neighbours_dirty(&mut self, position: ChunkPos) {
        for direction in [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::NEG_Y,
            IVec3::Z,
            IVec3::NEG_Z,
        ] {
            self.mark_chunk_dirty(position + direction);
        }
    }

    fn mark_chunk_dirty(&mut self, position: ChunkPos) {
        if let Some(chunk) = self.chunks.get_mut(&position) {
            chunk.flags_mut().insert(ChunkFlags::DIRTY);
            chunk.flags_mut().remove(ChunkFlags::MESHED);
            self.queue_dirty_chunk(position);
        }
    }

    fn queue_dirty_chunk(&mut self, position: ChunkPos) {
        if self.chunks.contains_key(&position) && self.queued_dirty_chunks.insert(position) {
            self.pending_dirty_chunks.push_back(position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_across_negative_chunk_boundaries() {
        let mut world = World::default();
        let position = IVec3::new(-1, 17, -16);

        assert_eq!(world.block(position), BlockId::AIR);
        assert_eq!(world.set_block(position, BlockId::STONE), BlockId::AIR);
        assert_eq!(world.block(position), BlockId::STONE);

        let (chunk_position, local) = coordinates::split_global(position);
        assert_eq!(chunk_position, IVec3::new(-1, 1, -1));
        assert_eq!(local, UVec3::new(15, 1, 0));
    }

    #[test]
    fn editing_a_chunk_border_marks_the_neighbour_dirty() {
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Chunk::empty());
        world.insert_chunk(IVec3::X, Chunk::empty());

        world.set_block(IVec3::new(15, 0, 0), BlockId::STONE);

        assert!(
            world
                .chunk(IVec3::X)
                .expect("neighbour chunk")
                .flags()
                .contains(ChunkFlags::DIRTY)
        );
    }

    #[test]
    fn player_edits_survive_chunk_unload_and_regeneration() {
        use crate::application::core::world::{TerrainDimensions, WorldGenerator};

        let generator = WorldGenerator::legacy_flat(TerrainDimensions::default(), 0);
        let placed_position = IVec3::new(2, 2, 2);
        let broken_position = IVec3::new(3, 1, 2);
        let chunk_position = coordinates::split_global(placed_position).0;
        let generated = generator.generate_chunk(chunk_position).unwrap();
        let mut world = World::default();
        world.insert_chunk(chunk_position, generated);

        assert_eq!(
            world.edit_block(placed_position, BlockId::COBBLESTONE),
            Some(BlockId::AIR)
        );
        assert_eq!(
            world.edit_block(broken_position, BlockId::AIR),
            Some(BlockId::GRASS)
        );
        assert_eq!(world.edited_block_count(), 2);
        world.remove_chunk(chunk_position);
        world.insert_chunk(
            chunk_position,
            generator.generate_chunk(chunk_position).unwrap(),
        );

        assert_eq!(world.block(placed_position), BlockId::COBBLESTONE);
        assert_eq!(world.block(broken_position), BlockId::AIR);
    }

    #[test]
    fn placing_a_source_over_a_current_records_a_persistent_edit() {
        use crate::application::core::world::{TerrainDimensions, WorldGenerator};

        let generator = WorldGenerator::legacy_flat(TerrainDimensions::default(), 0);
        let position = IVec3::new(2, 5, 2);
        let chunk_position = coordinates::split_global(position).0;
        let mut world = World::default();
        world.insert_chunk(
            chunk_position,
            generator.generate_chunk(chunk_position).unwrap(),
        );
        world.set_simulated_block(position, BlockId::WATER);

        assert_eq!(
            world.edit_block(position, BlockId::WATER),
            Some(BlockId::WATER)
        );
        assert!(
            world
                .persistent_edits(generator)
                .contains(&(position, BlockId::WATER))
        );
    }
}
