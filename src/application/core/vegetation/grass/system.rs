use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet},
    time::Duration,
};

use glam::{IVec3, UVec3};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::application::core::{
    blocks::BlockId,
    world::{CHUNK_SIZE, ChunkPos, World, split_global},
};

use super::directions::SPREAD_DIRECTIONS;

const RANDOM_TICK_INTERVAL: Duration = Duration::from_secs(55);
const MIN_DECAY_SECONDS: usize = 8;
const MAX_DECAY_SECONDS: usize = 16;
const RANDOM_TICK_BUDGET_PER_FRAME: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TimerKind {
    Spread,
    Decay,
}

#[derive(Clone, Copy)]
struct GrassTimer {
    deadline: Option<Duration>,
    kind: Option<TimerKind>,
    token: u64,
}

#[derive(Default)]
pub(crate) struct GrassSystem {
    elapsed: Duration,
    timers: FxHashMap<IVec3, GrassTimer>,
    positions_by_chunk: FxHashMap<ChunkPos, FxHashSet<IVec3>>,
    scheduled: BinaryHeap<Reverse<(Duration, u64, [i32; 3])>>,
    next_token: u64,
    random_state: u64,
}

impl GrassSystem {
    pub fn reset(&mut self, seed: u64) {
        *self = Self {
            random_state: seed.max(1),
            ..Self::default()
        };
    }

    pub fn register_chunk(&mut self, world: &mut World, chunk_position: ChunkPos) -> usize {
        self.unload_chunk(chunk_position);
        let Some(chunk) = world.chunk(chunk_position) else {
            return 0;
        };
        let mut grass = Vec::new();
        let origin = chunk_position * CHUNK_SIZE as i32;
        for x in 0..CHUNK_SIZE as u32 {
            for y in 0..CHUNK_SIZE as u32 {
                for z in 0..CHUNK_SIZE as u32 {
                    let local = UVec3::new(x, y, z);
                    if chunk.block(local) == BlockId::GRASS {
                        grass.push(origin + local.as_ivec3());
                    }
                }
            }
        }

        for &position in &grass {
            self.track(position);
        }
        grass
            .into_iter()
            .map(|position| self.refresh(world, position))
            .sum()
    }

    pub fn unload_chunk(&mut self, chunk_position: ChunkPos) {
        if let Some(positions) = self.positions_by_chunk.remove(&chunk_position) {
            for position in positions {
                self.timers.remove(&position);
            }
        }
    }

    pub fn on_block_changed(&mut self, world: &mut World, changed: IVec3) -> usize {
        let mut affected = HashSet::with_capacity(54);
        affected.insert(changed);
        affected.insert(changed - IVec3::Y);
        for target in [changed, changed - IVec3::Y] {
            for direction in SPREAD_DIRECTIONS {
                affected.insert(target - direction);
            }
        }
        affected
            .into_iter()
            .map(|position| self.refresh(world, position))
            .sum()
    }

    pub fn on_lighting_changed(&mut self, world: &mut World, chunk_position: ChunkPos) -> usize {
        let mut positions = Vec::new();
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let neighbour = chunk_position + IVec3::new(x, y, z);
                    if let Some(grass) = self.positions_by_chunk.get(&neighbour) {
                        positions.extend(grass.iter().copied());
                    }
                }
            }
        }
        positions
            .into_iter()
            .map(|position| self.refresh(world, position))
            .sum()
    }

    pub fn update(&mut self, delta_time: Duration, world: &mut World) -> usize {
        self.elapsed += delta_time;
        let mut mutations = 0;
        let mut processed = 0;
        while processed < RANDOM_TICK_BUDGET_PER_FRAME {
            let Some(Reverse((deadline, token, position))) = self.scheduled.peek().copied() else {
                break;
            };
            if deadline > self.elapsed {
                break;
            }
            self.scheduled.pop();
            let position = IVec3::from_array(position);
            let valid = self
                .timers
                .get(&position)
                .is_some_and(|timer| timer.token == token && timer.deadline == Some(deadline));
            if valid {
                let kind = self.timers.get(&position).and_then(|timer| timer.kind);
                mutations += self.execute_tick(world, position, kind);
                processed += 1;
            }
        }
        mutations
    }

    fn execute_tick(
        &mut self,
        world: &mut World,
        position: IVec3,
        kind: Option<TimerKind>,
    ) -> usize {
        self.clear_deadline(position);
        if world.block(position) != BlockId::GRASS {
            self.untrack(position);
            return 0;
        }

        if kind == Some(TimerKind::Decay) {
            return if self.must_decay(world, position) {
                self.decay(world, position)
            } else {
                self.refresh(world, position)
            };
        }
        if self.must_decay(world, position) {
            return self.refresh(world, position);
        }
        if !self.has_spread_target(world, position) {
            return 0;
        }

        let direction = SPREAD_DIRECTIONS[self.random_index(SPREAD_DIRECTIONS.len())];
        let target = position + direction;
        let mut mutations = 0;
        if self.can_receive_grass(world, target)
            && world.edit_block(target, BlockId::GRASS).is_some()
        {
            self.track(target);
            mutations += 1;
            mutations += self.on_block_changed(world, target);
        }
        mutations + self.refresh(world, position)
    }

    fn refresh(&mut self, world: &mut World, position: IVec3) -> usize {
        if world.block(position) != BlockId::GRASS {
            self.untrack(position);
            return 0;
        }
        self.track(position);
        if self.must_decay(world, position) {
            if !self.has_timer_kind(position, TimerKind::Decay) {
                self.schedule_decay(position);
            }
            return 0;
        }

        if self.has_spread_target(world, position) {
            if !self.has_timer_kind(position, TimerKind::Spread) {
                self.schedule_spread(position);
            }
        } else {
            self.clear_deadline(position);
        }
        0
    }

    fn decay(&mut self, world: &mut World, position: IVec3) -> usize {
        self.untrack(position);
        let changed = world.edit_block(position, BlockId::DIRT).is_some();
        if changed {
            self.wake_nearby_sources(world, position);
        }
        usize::from(changed)
    }

    fn must_decay(&self, world: &World, position: IVec3) -> bool {
        let above = world.block(position + IVec3::Y);
        world.skylight(position) == 0 || (above != BlockId::AIR && !above.is_liquid())
    }

    fn has_spread_target(&self, world: &World, position: IVec3) -> bool {
        SPREAD_DIRECTIONS
            .into_iter()
            .any(|direction| self.can_receive_grass(world, position + direction))
    }

    fn can_receive_grass(&self, world: &World, position: IVec3) -> bool {
        world.block(position) == BlockId::DIRT
            && world.skylight(position) > 0
            && world.block(position + IVec3::Y) == BlockId::AIR
    }

    fn wake_nearby_sources(&mut self, world: &World, dirt: IVec3) {
        for direction in SPREAD_DIRECTIONS {
            let source = dirt - direction;
            if world.block(source) == BlockId::GRASS && !self.must_decay(world, source) {
                self.track(source);
                if self
                    .timers
                    .get(&source)
                    .is_some_and(|timer| timer.deadline.is_none())
                {
                    self.schedule_spread(source);
                }
            }
        }
    }

    fn track(&mut self, position: IVec3) {
        if self.timers.contains_key(&position) {
            return;
        }
        let token = self.next_token();
        self.timers.insert(
            position,
            GrassTimer {
                deadline: None,
                kind: None,
                token,
            },
        );
        self.positions_by_chunk
            .entry(split_global(position).0)
            .or_default()
            .insert(position);
    }

    fn untrack(&mut self, position: IVec3) {
        self.timers.remove(&position);
        let chunk = split_global(position).0;
        if let Some(positions) = self.positions_by_chunk.get_mut(&chunk) {
            positions.remove(&position);
            if positions.is_empty() {
                self.positions_by_chunk.remove(&chunk);
            }
        }
    }

    fn schedule_spread(&mut self, position: IVec3) {
        self.schedule(position, TimerKind::Spread, RANDOM_TICK_INTERVAL);
    }

    fn schedule_decay(&mut self, position: IVec3) {
        let seconds =
            MIN_DECAY_SECONDS + self.random_index(MAX_DECAY_SECONDS - MIN_DECAY_SECONDS + 1);
        self.schedule(
            position,
            TimerKind::Decay,
            Duration::from_secs(seconds as u64),
        );
    }

    fn schedule(&mut self, position: IVec3, kind: TimerKind, delay: Duration) {
        let deadline = self.elapsed + delay;
        let token = self.next_token();
        if let Some(timer) = self.timers.get_mut(&position) {
            timer.deadline = Some(deadline);
            timer.kind = Some(kind);
            timer.token = token;
            self.scheduled
                .push(Reverse((deadline, token, position.to_array())));
        }
    }

    fn clear_deadline(&mut self, position: IVec3) {
        let token = self.next_token();
        if let Some(timer) = self.timers.get_mut(&position) {
            timer.deadline = None;
            timer.kind = None;
            timer.token = token;
        }
    }

    fn has_timer_kind(&self, position: IVec3, kind: TimerKind) -> bool {
        self.timers
            .get(&position)
            .is_some_and(|timer| timer.deadline.is_some() && timer.kind == Some(kind))
    }

    fn next_token(&mut self) -> u64 {
        self.next_token = self.next_token.wrapping_add(1);
        self.next_token
    }

    fn random_index(&mut self, length: usize) -> usize {
        let mut value = self.random_state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.random_state = value;
        (value as usize) % length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit_grass_world() -> (World, IVec3) {
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Default::default());
        let grass = IVec3::new(8, 8, 8);
        world.set_block(grass, BlockId::GRASS);
        world.set_skylight(grass, 15);
        (world, grass)
    }

    fn place_dirt_in_next_random_direction(world: &mut World, grass: IVec3, seed: u64) -> IVec3 {
        let mut predictor = GrassSystem::default();
        predictor.reset(seed);
        let target = grass + SPREAD_DIRECTIONS[predictor.random_index(SPREAD_DIRECTIONS.len())];
        world.set_block(target, BlockId::DIRT);
        world.set_skylight(target, 15);
        target
    }

    #[test]
    fn eligible_grass_spreads_only_after_fifty_five_seconds() {
        let (mut world, grass) = lit_grass_world();
        let mut system = GrassSystem::default();
        system.reset(7);
        let target = place_dirt_in_next_random_direction(&mut world, grass, 7);
        system.register_chunk(&mut world, IVec3::ZERO);

        assert_eq!(system.update(Duration::from_secs(54), &mut world), 0);
        assert_eq!(system.update(Duration::from_secs(1), &mut world), 1);
        assert_eq!(world.block(target), BlockId::GRASS);
    }

    #[test]
    fn timer_starts_when_conditions_become_valid_not_before() {
        let (mut world, grass) = lit_grass_world();
        let mut system = GrassSystem::default();
        system.reset(9);
        system.register_chunk(&mut world, IVec3::ZERO);
        system.update(Duration::from_secs(50), &mut world);

        let target = place_dirt_in_next_random_direction(&mut world, grass, 9);
        system.on_block_changed(&mut world, target);
        assert_eq!(system.update(Duration::from_secs(5), &mut world), 0);
        assert_eq!(system.update(Duration::from_secs(50), &mut world), 1);
    }

    #[test]
    fn random_tick_can_miss_when_only_one_direction_contains_dirt() {
        let (mut world, grass) = lit_grass_world();
        let mut system = GrassSystem::default();
        system.reset(13);
        let missed = system.random_index(SPREAD_DIRECTIONS.len());
        system.reset(13);
        let target = grass + SPREAD_DIRECTIONS[(missed + 1) % SPREAD_DIRECTIONS.len()];
        world.set_block(target, BlockId::DIRT);
        world.set_skylight(target, 15);
        system.register_chunk(&mut world, IVec3::ZERO);

        assert_eq!(system.update(RANDOM_TICK_INTERVAL, &mut world), 0);
        assert_eq!(world.block(target), BlockId::DIRT);
    }

    #[test]
    fn losing_eligibility_resets_the_full_timer() {
        let (mut world, grass) = lit_grass_world();
        let mut system = GrassSystem::default();
        system.reset(21);
        let target = place_dirt_in_next_random_direction(&mut world, grass, 21);
        system.register_chunk(&mut world, IVec3::ZERO);
        system.update(Duration::from_secs(50), &mut world);

        let cover = target + IVec3::Y;
        world.set_block(cover, BlockId::STONE);
        system.on_block_changed(&mut world, cover);
        system.update(Duration::from_secs(10), &mut world);
        world.set_block(cover, BlockId::AIR);
        system.on_block_changed(&mut world, cover);

        assert_eq!(system.update(Duration::from_secs(54), &mut world), 0);
        assert_eq!(world.block(target), BlockId::DIRT);
        assert_eq!(system.update(Duration::from_secs(1), &mut world), 1);
        assert_eq!(world.block(target), BlockId::GRASS);
    }

    #[test]
    fn covered_or_unlit_grass_decays_after_random_delay() {
        for covered in [false, true] {
            let (mut world, grass) = lit_grass_world();
            let mut system = GrassSystem::default();
            system.reset(11);
            system.register_chunk(&mut world, IVec3::ZERO);
            if covered {
                world.set_block(grass + IVec3::Y, BlockId::STONE);
                system.on_block_changed(&mut world, grass + IVec3::Y);
            } else {
                world.set_skylight(grass, 0);
                system.on_lighting_changed(&mut world, IVec3::ZERO);
            }
            assert_eq!(world.block(grass), BlockId::GRASS);
            system.update(Duration::from_secs(7), &mut world);
            assert_eq!(world.block(grass), BlockId::GRASS);
            system.update(Duration::from_secs(9), &mut world);
            assert_eq!(world.block(grass), BlockId::DIRT);
        }
    }

    #[test]
    fn grass_created_by_spreading_inherits_grass_behaviour() {
        let (mut world, grass) = lit_grass_world();
        let mut system = GrassSystem::default();
        system.reset(31);
        let target = place_dirt_in_next_random_direction(&mut world, grass, 31);
        system.register_chunk(&mut world, IVec3::ZERO);
        system.update(RANDOM_TICK_INTERVAL, &mut world);

        assert_eq!(world.block(target), BlockId::GRASS);
        assert!(system.timers.contains_key(&target));

        world.set_block(target + IVec3::Y, BlockId::STONE);
        system.on_block_changed(&mut world, target + IVec3::Y);
        system.update(Duration::from_secs(MAX_DECAY_SECONDS as u64), &mut world);
        assert_eq!(world.block(target), BlockId::DIRT);
    }

    #[test]
    fn decay_is_cancelled_when_conditions_recover() {
        let (mut world, grass) = lit_grass_world();
        let mut system = GrassSystem::default();
        system.reset(41);
        system.register_chunk(&mut world, IVec3::ZERO);

        let cover = grass + IVec3::Y;
        world.set_block(cover, BlockId::STONE);
        system.on_block_changed(&mut world, cover);
        system.update(Duration::from_secs(7), &mut world);
        world.set_block(cover, BlockId::AIR);
        system.on_block_changed(&mut world, cover);
        system.update(Duration::from_secs(MAX_DECAY_SECONDS as u64), &mut world);

        assert_eq!(world.block(grass), BlockId::GRASS);
    }

    #[test]
    fn transparent_liquids_do_not_kill_grass_below_them() {
        let (mut world, grass) = lit_grass_world();
        let system = GrassSystem::default();

        for liquid in [BlockId::WATER, BlockId::LAVA] {
            world.set_block(grass + IVec3::Y, liquid);
            assert!(!system.must_decay(&world, grass));
        }
    }
}
