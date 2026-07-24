use std::{cmp::Reverse, collections::BinaryHeap, time::Duration};

use glam::{IVec3, UVec3};
use rustc_hash::FxHashMap;

use crate::application::core::world::{
    BlockChange, CHUNK_SIZE, ChunkFlags, ChunkPos, FluidState, World, split_global,
};

use super::{FluidCell, FluidKind};

const HORIZONTAL_DIRECTIONS: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];
const ALL_DIRECTIONS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];
const EVENT_BUDGET_PER_FRAME: usize = 256;
const MUTATION_BUDGET_PER_FRAME: usize = 128;

type ScheduledEvent = Reverse<(Duration, u64, [i32; 3], u64)>;

#[derive(Default)]
pub(crate) struct FluidSystem {
    elapsed: Duration,
    cells: FxHashMap<IVec3, FluidCell>,
    positions_by_chunk: FxHashMap<ChunkPos, rustc_hash::FxHashSet<IVec3>>,
    scheduled: BinaryHeap<ScheduledEvent>,
    pending: FxHashMap<IVec3, (u64, Duration)>,
    next_sequence: u64,
    next_revision: u64,
}

impl FluidSystem {
    pub(crate) fn register_chunk(&mut self, world: &mut World, chunk_position: ChunkPos) -> usize {
        let Some(chunk) = world.chunk(chunk_position) else {
            return 0;
        };
        let mut sources = Vec::new();
        if chunk.flags().contains(ChunkFlags::FLUID_SOURCES) {
            let mut candidates = chunk.fluid_source_candidates().collect::<Vec<_>>();
            // Generated chunks build this compact list off-thread. Keep a cheap
            // compatibility fallback for manually assembled/legacy chunks.
            if candidates.is_empty() {
                for x in 0..CHUNK_SIZE as u32 {
                    for y in 0..CHUNK_SIZE as u32 {
                        for z in 0..CHUNK_SIZE as u32 {
                            let local = UVec3::new(x, y, z);
                            if FluidKind::from_block(chunk.block(local)).is_some() {
                                candidates.push(local);
                            }
                        }
                    }
                }
            }
            for local in candidates {
                let block = chunk.block(local);
                if let Some(kind) = FluidKind::from_block(block) {
                    let position = chunk_position * CHUNK_SIZE as i32 + local.as_ivec3();
                    let exposed = ALL_DIRECTIONS
                        .into_iter()
                        .any(|direction| world.block(position + direction) != block);
                    if exposed {
                        sources.push((position, kind));
                    }
                }
            }
        }
        let count = sources.len();
        for (position, kind) in sources {
            self.insert_source(world, position, kind, false);
            self.activate_generated_fall(world, position);
        }

        let origin = chunk_position * CHUNK_SIZE as i32;
        for x in 0..CHUNK_SIZE as i32 {
            for z in 0..CHUNK_SIZE as i32 {
                let source_above = origin + IVec3::new(x, CHUNK_SIZE as i32, z);
                self.activate_generated_fall(world, source_above);
            }
        }
        count
    }

    pub(crate) fn on_block_changed(&mut self, world: &mut World, change: BlockChange) {
        let previous_fluid = FluidKind::from_block(change.previous);
        let current_fluid = FluidKind::from_block(change.current);

        if previous_fluid.is_some() && previous_fluid != current_fluid {
            self.remove_cell(change.position);
            world.clear_fluid_state(change.position);
        }

        if let Some(kind) = current_fluid {
            self.insert_source(world, change.position, kind, true);
        }

        self.schedule_nearby(world, change.position);
    }

    pub(crate) fn update(&mut self, delta_time: Duration, world: &mut World) -> usize {
        self.elapsed += delta_time;
        let mut events = 0;
        let mut mutations = 0;

        while events < EVENT_BUDGET_PER_FRAME && mutations < MUTATION_BUDGET_PER_FRAME {
            let Some(Reverse((due, _, position, revision))) = self.scheduled.peek().copied() else {
                break;
            };
            if due > self.elapsed {
                break;
            }
            self.scheduled.pop();
            events += 1;

            let position = IVec3::from_array(position);
            if self.pending.get(&position) != Some(&(revision, due)) {
                continue;
            }
            self.pending.remove(&position);
            if self
                .cells
                .get(&position)
                .is_none_or(|cell| cell.revision != revision)
            {
                continue;
            }
            mutations += self.advance(world, position);
        }

        mutations
    }

    pub(crate) fn unload_chunks(&mut self, chunks: &[ChunkPos]) {
        for chunk in chunks {
            let Some(positions) = self.positions_by_chunk.remove(chunk) else {
                continue;
            };
            for position in positions {
                self.cells.remove(&position);
                self.pending.remove(&position);
            }
        }
    }

    fn insert_source(
        &mut self,
        world: &mut World,
        position: IVec3,
        kind: FluidKind,
        activate: bool,
    ) {
        let revision = self.next_revision();
        self.cells.insert(
            position,
            FluidCell {
                kind,
                level: 0,
                falling: false,
                source: true,
                revision,
            },
        );
        self.track_cell(position);
        if activate {
            world.set_fluid_state(position, FluidState::SOURCE);
            self.schedule(position, revision, kind.step_interval());
        }
    }

    fn advance(&mut self, world: &mut World, position: IVec3) -> usize {
        let Some(cell) = self.cells.get(&position).copied() else {
            return 0;
        };
        if world.block(position) != cell.kind.block() {
            self.remove_cell(position);
            world.clear_fluid_state(position);
            self.schedule_dependents(world, position, cell.kind);
            return 0;
        }

        if !cell.source && !self.is_supported(world, position, cell) {
            self.remove_cell(position);
            world.clear_fluid_state(position);
            let removed = world
                .set_simulated_block(position, crate::application::core::blocks::BlockId::AIR)
                .is_some();
            self.schedule_dependents(world, position, cell.kind);
            return usize::from(removed);
        }

        let below = position - IVec3::Y;
        if !world.contains_block(below) {
            self.schedule(position, cell.revision, cell.kind.step_interval());
            return 0;
        }
        if world.block(below) == crate::application::core::blocks::BlockId::AIR {
            return self.insert_flow(world, below, cell.kind, 0, true);
        }

        if cell.level >= cell.kind.maximum_level() {
            return 0;
        }

        let mut mutations = 0;
        let mut touches_unloaded = false;
        for direction in HORIZONTAL_DIRECTIONS {
            let target = position + direction;
            if !world.contains_block(target) {
                touches_unloaded = true;
                continue;
            }
            mutations += self.insert_flow(world, target, cell.kind, cell.level + 1, false);
        }
        if touches_unloaded {
            self.schedule(position, cell.revision, cell.kind.step_interval());
        }
        mutations
    }

    fn insert_flow(
        &mut self,
        world: &mut World,
        position: IVec3,
        kind: FluidKind,
        level: u8,
        falling: bool,
    ) -> usize {
        let current = world.block(position);
        if current == kind.block() {
            let should_upgrade = self.cells.get(&position).is_some_and(|cell| {
                !cell.source && (level < cell.level || (falling && !cell.falling))
            });
            if should_upgrade {
                let revision = self.next_revision();
                let state = if let Some(cell) = self.cells.get_mut(&position) {
                    cell.level = level;
                    cell.falling |= falling;
                    cell.revision = revision;
                    FluidState::new(cell.level, cell.falling)
                } else {
                    return 0;
                };
                world.set_fluid_state(position, state);
                self.schedule(position, revision, kind.step_interval());
            }
            return 0;
        }
        if current != crate::application::core::blocks::BlockId::AIR {
            return 0;
        }
        if world.set_simulated_block(position, kind.block()).is_none() {
            return 0;
        }

        let revision = self.next_revision();
        self.cells.insert(
            position,
            FluidCell {
                kind,
                level,
                falling,
                source: false,
                revision,
            },
        );
        self.track_cell(position);
        world.set_fluid_state(position, FluidState::new(level, falling));
        self.schedule(position, revision, kind.step_interval());
        1
    }

    fn is_supported(&self, world: &World, position: IVec3, cell: FluidCell) -> bool {
        if cell.falling {
            let above = position + IVec3::Y;
            return world.block(above) == cell.kind.block() && self.cells.contains_key(&above);
        }

        HORIZONTAL_DIRECTIONS.into_iter().any(|direction| {
            let neighbour = position + direction;
            world.block(neighbour) == cell.kind.block()
                && self.cells.get(&neighbour).is_some_and(|candidate| {
                    candidate.kind == cell.kind
                        && (candidate.source || candidate.level < cell.level)
                })
        })
    }

    fn schedule_nearby(&mut self, world: &World, position: IVec3) {
        for candidate in [
            position,
            position + IVec3::Y,
            position - IVec3::Y,
            position + IVec3::X,
            position - IVec3::X,
            position + IVec3::Z,
            position - IVec3::Z,
        ] {
            let Some(cell) = self.cells.get(&candidate).copied() else {
                continue;
            };
            if world.block(candidate) == cell.kind.block() {
                self.schedule(candidate, cell.revision, cell.kind.step_interval());
            }
        }
    }

    fn schedule_dependents(&mut self, world: &World, position: IVec3, kind: FluidKind) {
        for candidate in [
            position - IVec3::Y,
            position + IVec3::X,
            position - IVec3::X,
            position + IVec3::Z,
            position - IVec3::Z,
        ] {
            let Some(cell) = self.cells.get(&candidate).copied() else {
                continue;
            };
            if cell.kind == kind && world.block(candidate) == kind.block() {
                self.schedule(candidate, cell.revision, kind.step_interval());
            }
        }
    }

    fn schedule(&mut self, position: IVec3, revision: u64, delay: Duration) {
        let due = self.elapsed + delay;
        if self
            .pending
            .get(&position)
            .is_some_and(|&(queued_revision, queued_due)| {
                queued_revision == revision && queued_due <= due
            })
        {
            return;
        }
        self.pending.insert(position, (revision, due));
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.scheduled
            .push(Reverse((due, sequence, position.to_array(), revision)));
    }

    fn next_revision(&mut self) -> u64 {
        let revision = self.next_revision;
        self.next_revision = self.next_revision.wrapping_add(1);
        revision
    }

    fn activate_generated_fall(&mut self, world: &World, position: IVec3) {
        let below = position - IVec3::Y;
        if !world.contains_block(below)
            || world.block(below) != crate::application::core::blocks::BlockId::AIR
        {
            return;
        }
        let Some(cell) = self.cells.get(&position).copied() else {
            return;
        };
        if cell.source && world.block(position) == cell.kind.block() {
            self.schedule(position, cell.revision, cell.kind.step_interval());
        }
    }

    fn track_cell(&mut self, position: IVec3) {
        self.positions_by_chunk
            .entry(split_global(position).0)
            .or_default()
            .insert(position);
    }

    fn remove_cell(&mut self, position: IVec3) {
        self.cells.remove(&position);
        self.pending.remove(&position);
        let chunk = split_global(position).0;
        let remove_chunk_entry = self
            .positions_by_chunk
            .get_mut(&chunk)
            .is_some_and(|positions| {
                positions.remove(&position);
                positions.is_empty()
            });
        if remove_chunk_entry {
            self.positions_by_chunk.remove(&chunk);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::core::{
        blocks::BlockId,
        world::{CHUNK_SIZE, World},
    };

    fn empty_world() -> World {
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Default::default());
        world
    }

    fn add_floor(world: &mut World, y: i32) {
        for x in 0..CHUNK_SIZE as i32 {
            for z in 0..CHUNK_SIZE as i32 {
                world.set_block(IVec3::new(x, y, z), BlockId::STONE);
            }
        }
    }

    fn add_source(system: &mut FluidSystem, world: &mut World, position: IVec3, block: BlockId) {
        let previous = world.edit_block(position, block).unwrap();
        system.on_block_changed(
            world,
            BlockChange {
                position,
                previous,
                current: block,
            },
        );
    }

    #[test]
    fn water_waits_half_a_second_and_prefers_falling() {
        let mut world = empty_world();
        let mut system = FluidSystem::default();
        let source = IVec3::new(8, 5, 8);
        add_source(&mut system, &mut world, source, BlockId::WATER);

        assert_eq!(system.update(Duration::from_millis(499), &mut world), 0);
        assert_eq!(world.block(source - IVec3::Y), BlockId::AIR);
        assert_eq!(system.update(Duration::from_millis(1), &mut world), 1);
        assert_eq!(world.block(source - IVec3::Y), BlockId::WATER);
        assert_eq!(world.block(source + IVec3::X), BlockId::AIR);
    }

    #[test]
    fn falling_water_is_not_limited_by_horizontal_reach() {
        let mut world = empty_world();
        let mut system = FluidSystem::default();
        let source = IVec3::new(8, 15, 8);
        add_source(&mut system, &mut world, source, BlockId::WATER);

        for _ in 0..10 {
            system.update(Duration::from_millis(500), &mut world);
        }

        let falling = source - IVec3::Y * 10;
        assert_eq!(world.block(falling), BlockId::WATER);
        assert!(world.fluid_state(falling).unwrap().is_falling());
        assert_eq!(world.fluid_state(falling).unwrap().level(), 0);
    }

    #[test]
    fn water_and_lava_respect_their_horizontal_reach() {
        let mut water_world = empty_world();
        add_floor(&mut water_world, 0);
        let mut water = FluidSystem::default();
        let water_source = IVec3::new(3, 1, 3);
        add_source(&mut water, &mut water_world, water_source, BlockId::WATER);
        assert_eq!(
            water_world.fluid_state(water_source),
            Some(FluidState::SOURCE)
        );
        for _ in 0..8 {
            water.update(Duration::from_millis(500), &mut water_world);
        }
        assert_eq!(
            water_world
                .fluid_state(water_source + IVec3::X)
                .unwrap()
                .level(),
            1
        );
        assert_eq!(
            water_world.block(water_source + IVec3::X * 8),
            BlockId::WATER
        );
        assert_eq!(water_world.block(water_source + IVec3::X * 9), BlockId::AIR);

        let mut lava_world = empty_world();
        add_floor(&mut lava_world, 0);
        let mut lava = FluidSystem::default();
        let lava_source = IVec3::new(3, 1, 3);
        add_source(&mut lava, &mut lava_world, lava_source, BlockId::LAVA);
        for _ in 0..5 {
            lava.update(Duration::from_secs(1), &mut lava_world);
        }
        assert_eq!(lava_world.block(lava_source + IVec3::X * 5), BlockId::LAVA);
        assert_eq!(lava_world.block(lava_source + IVec3::X * 6), BlockId::AIR);
    }

    #[test]
    fn lava_waits_one_second_before_spreading() {
        let mut world = empty_world();
        add_floor(&mut world, 0);
        let mut system = FluidSystem::default();
        let source = IVec3::new(8, 1, 8);
        add_source(&mut system, &mut world, source, BlockId::LAVA);

        assert_eq!(system.update(Duration::from_millis(999), &mut world), 0);
        assert_eq!(world.block(source + IVec3::X), BlockId::AIR);
        assert!(system.update(Duration::from_millis(1), &mut world) > 0);
        assert_eq!(world.block(source + IVec3::X), BlockId::LAVA);
        assert_eq!(world.fluid_state(source + IVec3::X).unwrap().level(), 1);
    }

    #[test]
    fn currents_recede_after_their_source_is_removed() {
        let mut world = empty_world();
        add_floor(&mut world, 0);
        let mut system = FluidSystem::default();
        let source = IVec3::new(8, 1, 8);
        add_source(&mut system, &mut world, source, BlockId::WATER);
        system.update(Duration::from_millis(500), &mut world);
        assert_eq!(world.block(source + IVec3::X), BlockId::WATER);

        let previous = world.edit_block(source, BlockId::AIR).unwrap();
        system.on_block_changed(
            &mut world,
            BlockChange {
                position: source,
                previous,
                current: BlockId::AIR,
            },
        );
        system.update(Duration::from_millis(500), &mut world);

        assert_eq!(world.block(source + IVec3::X), BlockId::AIR);
    }

    #[test]
    fn a_fall_renews_horizontal_reach_for_infinite_terrain_flow() {
        let mut world = empty_world();
        add_floor(&mut world, 0);
        for x in 0..=4 {
            for z in 0..CHUNK_SIZE as i32 {
                world.set_block(IVec3::new(x, 1, z), BlockId::STONE);
            }
        }

        let mut system = FluidSystem::default();
        let source = IVec3::new(1, 2, 8);
        add_source(&mut system, &mut world, source, BlockId::WATER);
        for _ in 0..14 {
            system.update(Duration::from_millis(500), &mut world);
        }

        assert_eq!(world.block(IVec3::new(13, 1, 8)), BlockId::WATER);
    }

    #[test]
    fn generated_ocean_registers_only_its_reactive_shell_and_stays_dormant() {
        let mut world = empty_world();
        for x in 0..CHUNK_SIZE as i32 {
            for y in 0..CHUNK_SIZE as i32 {
                for z in 0..CHUNK_SIZE as i32 {
                    world.set_block(IVec3::new(x, y, z), BlockId::WATER);
                }
            }
        }

        let mut system = FluidSystem::default();
        let registered = system.register_chunk(&mut world, IVec3::ZERO);

        assert!(registered < CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE);
        assert!(system.scheduled.is_empty());
        assert_eq!(system.update(Duration::from_secs(10), &mut world), 0);
    }

    #[test]
    fn generated_liquid_falls_when_the_chunk_below_arrives_later() {
        let mut world = World::default();
        let upper_chunk = IVec3::Y;
        world.insert_chunk(upper_chunk, Default::default());
        let source = IVec3::new(8, CHUNK_SIZE as i32, 8);
        world.set_block(source, BlockId::WATER);
        world
            .chunk_mut(upper_chunk)
            .unwrap()
            .flags_mut()
            .insert(ChunkFlags::FLUID_SOURCES);

        let mut system = FluidSystem::default();
        system.register_chunk(&mut world, upper_chunk);
        assert!(system.scheduled.is_empty());

        world.insert_chunk(IVec3::ZERO, Default::default());
        system.register_chunk(&mut world, IVec3::ZERO);
        assert_eq!(system.update(Duration::from_millis(500), &mut world), 1);
        assert_eq!(world.block(source - IVec3::Y), BlockId::WATER);
        assert!(world.fluid_state(source - IVec3::Y).unwrap().is_falling());
    }
}
