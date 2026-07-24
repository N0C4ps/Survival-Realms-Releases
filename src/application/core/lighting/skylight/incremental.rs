use std::collections::VecDeque;

use glam::{IVec2, IVec3};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::application::core::{
    blocks::BlockRegistry,
    world::{BlockChange, CHUNK_SIZE, ChunkPos, World, WorldGenerator, split_global},
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
const UPDATE_RADIUS: i32 = LightLevel::FULL.value() as i32;

pub(super) fn apply_block_change(
    world: &mut World,
    registry: &BlockRegistry,
    generator: WorldGenerator,
    change: BlockChange,
) {
    let previous_opaque = registry.get(change.previous).properties().is_opaque();
    let current_opaque = registry.get(change.current).properties().is_opaque();
    if previous_opaque == current_opaque {
        return;
    }

    let Some((minimum_y, maximum_y)) = loaded_vertical_bounds(world) else {
        return;
    };
    let mut sky = DirectSkyCache::new(maximum_y, generator);
    let mut removed = VecDeque::new();
    let mut additions = VecDeque::new();
    let mut changed_chunks = FxHashSet::default();

    rebuild_edited_column(
        world,
        registry,
        change.position,
        minimum_y,
        maximum_y,
        &mut sky,
        &mut removed,
        &mut additions,
        &mut changed_chunks,
    );
    remove_dependent_light(
        world,
        registry,
        &mut sky,
        &mut removed,
        &mut additions,
        &mut changed_chunks,
        change.position,
    );
    spread_new_light(
        world,
        registry,
        &mut sky,
        &mut additions,
        &mut changed_chunks,
        change.position,
    );

    world.mark_lighting_chunks_dirty(&changed_chunks);
    tracing::debug!(
        position = ?change.position,
        changed_chunks = changed_chunks.len(),
        "incremental skylight updated"
    );
}

#[allow(clippy::too_many_arguments)]
fn rebuild_edited_column(
    world: &mut World,
    registry: &BlockRegistry,
    edited: IVec3,
    minimum_y: i32,
    maximum_y: i32,
    sky: &mut DirectSkyCache,
    removed: &mut VecDeque<(IVec3, u8)>,
    additions: &mut VecDeque<IVec3>,
    changed_chunks: &mut FxHashSet<ChunkPos>,
) {
    let column = IVec2::new(edited.x, edited.z);
    let sky_floor = sky.floor(world, registry, column);

    let minimum_y = minimum_y.max(edited.y - UPDATE_RADIUS);
    let maximum_y = maximum_y.min(edited.y + UPDATE_RADIUS);
    for y in minimum_y..=maximum_y {
        let position = IVec3::new(edited.x, y, edited.z);
        if world.chunk(split_global(position).0).is_none() {
            continue;
        }

        if y >= sky_floor {
            set_level(world, position, LightLevel::FULL.value(), changed_chunks);
            if !is_opaque(world, registry, position) {
                additions.push_back(position);
            }
            continue;
        }

        let previous = world.skylight(position);
        if previous > 0 {
            set_level(world, position, 0, changed_chunks);
            removed.push_back((position, previous));
        }
        for direction in DIRECTIONS {
            let neighbour = position + direction;
            if world.skylight(neighbour) > 0 {
                additions.push_back(neighbour);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn remove_dependent_light(
    world: &mut World,
    registry: &BlockRegistry,
    sky: &mut DirectSkyCache,
    removed: &mut VecDeque<(IVec3, u8)>,
    additions: &mut VecDeque<IVec3>,
    changed_chunks: &mut FxHashSet<ChunkPos>,
    edited: IVec3,
) {
    while let Some((position, removed_level)) = removed.pop_front() {
        for direction in DIRECTIONS {
            let neighbour = position + direction;
            if !inside_update_radius(neighbour, edited) {
                continue;
            }
            let neighbour_level = world.skylight(neighbour);
            if neighbour_level == 0 {
                continue;
            }

            if sky.contains(world, registry, neighbour) {
                set_level(world, neighbour, LightLevel::FULL.value(), changed_chunks);
                additions.push_back(neighbour);
            } else if neighbour_level < removed_level {
                set_level(world, neighbour, 0, changed_chunks);
                removed.push_back((neighbour, neighbour_level));
            } else {
                additions.push_back(neighbour);
            }
        }
    }
}

fn spread_new_light(
    world: &mut World,
    registry: &BlockRegistry,
    sky: &mut DirectSkyCache,
    additions: &mut VecDeque<IVec3>,
    changed_chunks: &mut FxHashSet<ChunkPos>,
    edited: IVec3,
) {
    while let Some(position) = additions.pop_front() {
        if !inside_update_radius(position, edited) {
            continue;
        }
        if is_opaque(world, registry, position) {
            continue;
        }
        let source_level = world.skylight(position);
        if source_level <= 1 {
            continue;
        }
        let propagated = source_level - 1;

        for direction in DIRECTIONS {
            let neighbour = position + direction;
            if !inside_update_radius(neighbour, edited) {
                continue;
            }
            if is_opaque(world, registry, neighbour) {
                continue;
            }
            let desired = if sky.contains(world, registry, neighbour) {
                LightLevel::FULL.value()
            } else {
                propagated
            };
            if world.skylight(neighbour) >= desired {
                continue;
            }
            if set_level(world, neighbour, desired, changed_chunks) {
                additions.push_back(neighbour);
            }
        }
    }
}

fn inside_update_radius(position: IVec3, edited: IVec3) -> bool {
    let distance = (position - edited).abs();
    distance.x + distance.y + distance.z <= UPDATE_RADIUS
}

fn set_level(
    world: &mut World,
    position: IVec3,
    level: u8,
    changed_chunks: &mut FxHashSet<ChunkPos>,
) -> bool {
    let Some(previous) = world.set_skylight(position, level) else {
        return false;
    };
    if previous == level {
        return false;
    }
    changed_chunks.insert(split_global(position).0);
    true
}

fn is_opaque(world: &World, registry: &BlockRegistry, position: IVec3) -> bool {
    let chunk = split_global(position).0;
    world.chunk(chunk).is_none() || registry.get(world.block(position)).properties().is_opaque()
}

fn loaded_vertical_bounds(world: &World) -> Option<(i32, i32)> {
    let minimum_chunk = world.chunks().map(|(position, _)| position.y).min()?;
    let maximum_chunk = world.chunks().map(|(position, _)| position.y).max()?;
    Some((
        minimum_chunk * CHUNK_SIZE as i32,
        (maximum_chunk + 1) * CHUNK_SIZE as i32 - 1,
    ))
}

struct DirectSkyCache {
    maximum_y: i32,
    generator: WorldGenerator,
    floors: FxHashMap<IVec2, i32>,
}

impl DirectSkyCache {
    fn new(maximum_y: i32, generator: WorldGenerator) -> Self {
        Self {
            maximum_y,
            generator,
            floors: FxHashMap::default(),
        }
    }

    fn contains(&mut self, world: &World, registry: &BlockRegistry, position: IVec3) -> bool {
        position.y >= self.floor(world, registry, IVec2::new(position.x, position.z))
    }

    fn floor(&mut self, world: &World, registry: &BlockRegistry, column: IVec2) -> i32 {
        *self.floors.entry(column).or_insert_with(|| {
            let original_surface = self.generator.surface_height(column.x, column.y);
            (original_surface..=self.maximum_y)
                .rev()
                .find(|&y| is_opaque(world, registry, IVec3::new(column.x, y, column.y)))
                .unwrap_or(original_surface)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::core::{
        blocks::BlockId,
        lighting::skylight::SkylightSystem,
        world::{TerrainDimensions, WorldGenerator},
    };

    #[test]
    fn digging_a_shaft_updates_only_from_each_edit() {
        let registry = BlockRegistry::builtin();
        let generator = WorldGenerator::legacy_flat(TerrainDimensions::new(1, 1, 2, 1), 0);
        let mut world = World::default();
        for chunk_y in -2..=0 {
            let chunk_position = IVec3::new(0, chunk_y, 0);
            world.insert_chunk(
                chunk_position,
                generator.generate_chunk(chunk_position).unwrap(),
            );
        }
        SkylightSystem::rebuild(&mut world, &registry, generator);

        for y in (-14..=1).rev() {
            let position = IVec3::new(0, y, 0);
            let previous = world.edit_block(position, BlockId::AIR).unwrap();
            apply_block_change(
                &mut world,
                &registry,
                generator,
                BlockChange {
                    position,
                    previous,
                    current: BlockId::AIR,
                },
            );
        }

        assert_eq!(world.skylight(IVec3::new(0, 1, 0)), 15);
        assert_eq!(world.skylight(IVec3::new(0, -14, 0)), 0);
    }
}
