use std::time::Duration;

use glam::{IVec3, UVec3, Vec3};
use rustc_hash::FxHashSet;

use crate::application::core::{
    blocks::{BlockId, BlockRegistry},
    world::{CHUNK_SIZE, ChunkPos, World, split_global},
};

use super::gravity::Gravity;

/// How many cells straight down we're willing to scan for solid ground
/// before giving up and just letting the block land where the scan stopped.
const FALL_SCAN_LIMIT: i32 = 512;

fn is_affected_by_gravity(block: BlockId) -> bool {
    matches!(block, BlockId::SAND | BlockId::GRAVEL)
}

/// A sand/gravel block that has left the voxel grid and is sliding down to
/// its resting position under gravity, rendered at a fractional height
/// instead of jumping from cell to cell.
struct FallingEntity {
    x: i32,
    z: i32,
    y: f32,
    velocity: f32,
    target_y: i32,
    block: BlockId,
    skylight: u8,
}

/// A falling block's current visual state, for the renderer to draw.
#[derive(Clone, Copy)]
pub(crate) struct FallingBlockInstance {
    pub position: Vec3,
    pub block: BlockId,
    pub skylight: u8,
}

/// Makes sand and gravel fall toward the nearest solid support, Minecraft-style.
///
/// A block is stable when it rests directly on a solid block; water and lava
/// are non-solid, so they never hold a falling block up. Losing support -
/// either the block below being broken or a block being placed with nothing
/// solid underneath it - pulls the block out of the voxel grid and slides it
/// down smoothly under gravity until it reaches its resting cell.
#[derive(Default)]
pub(crate) struct FallingBlockSystem {
    falling: Vec<FallingEntity>,
}

impl FallingBlockSystem {
    /// Scans a freshly loaded chunk for sand/gravel left floating by world
    /// generation (overhangs, carved caves, ...) and starts them falling.
    pub(crate) fn register_chunk(
        &mut self,
        world: &mut World,
        blocks: &BlockRegistry,
        chunk_position: ChunkPos,
    ) {
        let Some(chunk) = world.chunk(chunk_position) else {
            return;
        };
        let origin = chunk_position * CHUNK_SIZE as i32;
        let mut candidates = Vec::new();
        for x in 0..CHUNK_SIZE as u32 {
            for y in 0..CHUNK_SIZE as u32 {
                for z in 0..CHUNK_SIZE as u32 {
                    let local = UVec3::new(x, y, z);
                    if is_affected_by_gravity(chunk.block(local)) {
                        candidates.push(origin + local.as_ivec3());
                    }
                }
            }
        }
        for position in candidates {
            self.check(world, blocks, position);
        }
    }

    /// Reacts to a block change at `position`: the changed block itself may
    /// now be unsupported (freshly placed sand/gravel), and whatever sits
    /// directly above it may have just lost its support.
    pub(crate) fn on_block_changed(
        &mut self,
        world: &mut World,
        blocks: &BlockRegistry,
        position: IVec3,
    ) {
        self.check(world, blocks, position);
        self.check(world, blocks, position + IVec3::Y);
    }

    pub(crate) fn update(
        &mut self,
        delta_time: Duration,
        world: &mut World,
        blocks: &BlockRegistry,
    ) -> usize {
        let delta = delta_time.as_secs_f32().min(0.05);
        let gravity = Gravity::default();
        let mut mutations = 0;
        let mut index = 0;

        while index < self.falling.len() {
            let landed = {
                let entity = &mut self.falling[index];
                let offset = gravity.integrate(&mut entity.velocity, delta);
                entity.y += offset;
                entity.skylight =
                    world.skylight(IVec3::new(entity.x, entity.y.floor() as i32, entity.z));
                entity.y <= entity.target_y as f32
            };

            if landed {
                let entity = self.falling.swap_remove(index);
                let position = IVec3::new(entity.x, entity.target_y, entity.z);
                world.set_simulated_block(position, entity.block);
                mutations += 1;
                self.check(world, blocks, position + IVec3::Y);
            } else {
                index += 1;
            }
        }

        mutations
    }

    pub(crate) fn unload_chunks(&mut self, chunks: &[ChunkPos]) {
        if chunks.is_empty() {
            return;
        }
        let removed: FxHashSet<ChunkPos> = chunks.iter().copied().collect();
        self.falling.retain(|entity| {
            let column = split_global(IVec3::new(entity.x, entity.y.floor() as i32, entity.z)).0;
            !removed.contains(&column)
        });
    }

    pub(crate) fn instances(&self) -> impl Iterator<Item = FallingBlockInstance> + '_ {
        self.falling.iter().map(|entity| FallingBlockInstance {
            position: Vec3::new(entity.x as f32, entity.y, entity.z as f32),
            block: entity.block,
            skylight: entity.skylight,
        })
    }

    fn check(&mut self, world: &mut World, blocks: &BlockRegistry, position: IVec3) {
        let block = world.block(position);
        if !is_affected_by_gravity(block) {
            return;
        }
        let Some(mut landing_y) = find_landing(world, blocks, position) else {
            return;
        };

        // If another falling block is already headed for this column, land
        // on top of it instead of both aiming for the same resting cell.
        let reserved = self
            .falling
            .iter()
            .filter(|entity| entity.x == position.x && entity.z == position.z)
            .map(|entity| entity.target_y + 1)
            .max();
        if let Some(reserved) = reserved {
            landing_y = landing_y.max(reserved);
        }
        if landing_y >= position.y {
            return;
        }

        world.set_simulated_block(position, BlockId::AIR);
        self.falling.push(FallingEntity {
            x: position.x,
            z: position.z,
            y: position.y as f32,
            velocity: 0.0,
            target_y: landing_y,
            block,
            skylight: world.skylight(position),
        });
        self.check(world, blocks, position + IVec3::Y);
    }
}

fn find_landing(world: &World, blocks: &BlockRegistry, position: IVec3) -> Option<i32> {
    let mut y = position.y;
    loop {
        let below = IVec3::new(position.x, y - 1, position.z);
        if !world.contains_block(below) {
            return None;
        }
        if blocks.get(world.block(below)).properties().is_solid() {
            return Some(y);
        }
        y -= 1;
        if position.y - y > FALL_SCAN_LIMIT {
            return Some(y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_world(floor_y: i32) -> World {
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Default::default());
        for x in 0..CHUNK_SIZE as i32 {
            for z in 0..CHUNK_SIZE as i32 {
                world.set_block(IVec3::new(x, floor_y, z), BlockId::STONE);
            }
        }
        world
    }

    fn settle(system: &mut FallingBlockSystem, world: &mut World, blocks: &BlockRegistry) -> usize {
        let mut mutations = 0;
        for _ in 0..200 {
            mutations += system.update(Duration::from_millis(50), world, blocks);
        }
        mutations
    }

    #[test]
    fn sand_stays_put_on_solid_ground() {
        let mut world = floor_world(0);
        let blocks = BlockRegistry::builtin();
        let mut system = FallingBlockSystem::default();
        let position = IVec3::new(8, 1, 8);
        world.set_block(position, BlockId::SAND);

        system.on_block_changed(&mut world, &blocks, position);
        assert_eq!(settle(&mut system, &mut world, &blocks), 0);
        assert_eq!(world.block(position), BlockId::SAND);
        assert_eq!(system.instances().count(), 0);
    }

    #[test]
    fn gravel_falls_when_the_block_below_is_broken() {
        // Bedrock at y=0, a removable support block at y=1, gravel resting
        // on it at y=2. Breaking the support should send the gravel down to
        // rest directly on the bedrock.
        let mut world = floor_world(0);
        let blocks = BlockRegistry::builtin();
        let mut system = FallingBlockSystem::default();
        let support = IVec3::new(8, 1, 8);
        let position = IVec3::new(8, 2, 8);
        world.set_block(support, BlockId::STONE);
        world.set_block(position, BlockId::GRAVEL);
        system.on_block_changed(&mut world, &blocks, position);
        settle(&mut system, &mut world, &blocks);
        assert_eq!(world.block(position), BlockId::GRAVEL);

        world.set_block(support, BlockId::AIR);
        system.on_block_changed(&mut world, &blocks, support);

        // The block leaves the grid immediately and slides down smoothly
        // instead of jumping straight to its resting cell.
        assert_eq!(world.block(position), BlockId::AIR);
        assert_eq!(system.instances().count(), 1);
        let mid_flight = system.instances().next().unwrap();
        assert!(mid_flight.position.y > support.y as f32);

        settle(&mut system, &mut world, &blocks);
        assert_eq!(world.block(position), BlockId::AIR);
        assert_eq!(world.block(support), BlockId::GRAVEL);
        assert_eq!(system.instances().count(), 0);
    }

    #[test]
    fn sand_falls_through_water_and_lava_instead_of_resting_on_them() {
        for liquid in [BlockId::WATER, BlockId::LAVA] {
            let mut world = floor_world(0);
            let blocks = BlockRegistry::builtin();
            let mut system = FallingBlockSystem::default();
            let position = IVec3::new(8, 2, 8);
            world.set_block(position - IVec3::Y, liquid);
            world.set_block(position, BlockId::SAND);

            system.on_block_changed(&mut world, &blocks, position);
            settle(&mut system, &mut world, &blocks);

            assert_eq!(world.block(position - IVec3::Y), BlockId::SAND);
        }
    }

    #[test]
    fn stacked_gravel_falls_one_after_another() {
        // Bedrock at y=0, a removable support block at y=1, two stacked
        // gravel blocks above it. Both should fall and land stacked
        // directly on the bedrock, not on top of each other mid-air.
        let mut world = floor_world(0);
        let blocks = BlockRegistry::builtin();
        let mut system = FallingBlockSystem::default();
        let support = IVec3::new(8, 1, 8);
        let base = IVec3::new(8, 2, 8);
        world.set_block(support, BlockId::STONE);
        world.set_block(base, BlockId::GRAVEL);
        world.set_block(base + IVec3::Y, BlockId::GRAVEL);

        world.set_block(support, BlockId::AIR);
        system.on_block_changed(&mut world, &blocks, support);

        assert_eq!(system.instances().count(), 2);
        settle(&mut system, &mut world, &blocks);

        assert_eq!(world.block(support), BlockId::GRAVEL);
        assert_eq!(world.block(base), BlockId::GRAVEL);
        assert_eq!(world.block(base + IVec3::Y), BlockId::AIR);
        assert_eq!(system.instances().count(), 0);
    }

    #[test]
    fn placing_sand_over_nothing_immediately_starts_a_fall() {
        let mut world = floor_world(0);
        let blocks = BlockRegistry::builtin();
        let mut system = FallingBlockSystem::default();
        let position = IVec3::new(8, 8, 8);
        world.set_block(position, BlockId::SAND);

        system.on_block_changed(&mut world, &blocks, position);
        assert_eq!(world.block(position), BlockId::AIR);
        assert_eq!(system.instances().count(), 1);

        settle(&mut system, &mut world, &blocks);
        assert_eq!(world.block(IVec3::new(8, 1, 8)), BlockId::SAND);
    }
}
