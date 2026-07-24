use std::collections::VecDeque;

use glam::{IVec2, UVec3};
use rustc_hash::FxHashMap;

use crate::application::core::{
    blocks::BlockRegistry,
    world::{CHUNK_SIZE, World, WorldGenerator},
};

use super::{LightLevel, heightmap, propagation};

pub struct SkylightSystem;

impl SkylightSystem {
    pub fn rebuild(world: &mut World, registry: &BlockRegistry, generator: WorldGenerator) {
        let heightmap = heightmap::build(world, registry);
        let chunk_positions: Vec<_> = world.chunks().map(|(&position, _)| position).collect();
        let mut sources = VecDeque::new();
        let mut lighting_columns = FxHashMap::default();
        world.clear_skylight();

        for chunk_position in chunk_positions {
            let origin = chunk_position * CHUNK_SIZE as i32;
            for x in 0..CHUNK_SIZE as u32 {
                for z in 0..CHUNK_SIZE as u32 {
                    let column_position = origin + UVec3::new(x, 0, z).as_ivec3();
                    let column = IVec2::new(column_position.x, column_position.z);
                    let (sky_floor, propagation_ceiling) =
                        *lighting_columns.entry(column).or_insert_with(|| {
                            let original_surface =
                                generator.surface_height(column_position.x, column_position.z);
                            let floor = heightmap::surface_at(&heightmap, column_position)
                                .unwrap_or(original_surface)
                                .max(original_surface);
                            let ceiling = propagation_ceiling(&heightmap, generator, column, floor);
                            (floor, ceiling)
                        });

                    for y in 0..CHUNK_SIZE as u32 {
                        let position = origin + UVec3::new(x, y, z).as_ivec3();
                        let receives_sky = position.y >= sky_floor;

                        if receives_sky {
                            world.set_skylight(position, LightLevel::FULL.value());
                            let transparent =
                                !registry.get(world.block(position)).properties().is_opaque();
                            if transparent && position.y < propagation_ceiling {
                                sources.push_back(position);
                            }
                        }
                    }
                }
            }
        }

        let _ = propagation::spread(world, registry, sources);
        world.mark_all_dirty();
        tracing::info!(columns = heightmap.len(), "skylight rebuilt");
    }
}

fn propagation_ceiling(
    heightmap: &heightmap::Heightmap,
    generator: WorldGenerator,
    column: IVec2,
    floor: i32,
) -> i32 {
    let neighbouring_floor = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y]
        .into_iter()
        .map(|direction| column + direction)
        .map(|neighbour| {
            let original = generator.surface_height(neighbour.x, neighbour.y);
            heightmap
                .get(&neighbour)
                .copied()
                .unwrap_or(original)
                .max(original)
        })
        .max()
        .unwrap_or(floor);

    (floor + 1).max(neighbouring_floor)
}

#[cfg(test)]
mod tests {
    use glam::IVec3;

    use super::*;
    use crate::application::core::world::WorldGenerator;
    use crate::application::core::{blocks::BlockId, world::TerrainDimensions};

    #[test]
    fn surface_is_fully_lit_and_buried_blocks_are_dark() {
        let registry = BlockRegistry::builtin();
        let generator = WorldGenerator::legacy_flat(TerrainDimensions::new(1, 1, 1, 1), 0);
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, generator.generate_chunk(IVec3::ZERO).unwrap());
        world.insert_chunk(
            IVec3::NEG_Y,
            generator.generate_chunk(IVec3::NEG_Y).unwrap(),
        );

        SkylightSystem::rebuild(&mut world, &registry, generator);

        assert_eq!(world.skylight(IVec3::new(0, 1, 0)), 15);
        assert_eq!(world.skylight(IVec3::new(0, 0, 0)), 0);
    }

    #[test]
    fn light_reaches_zero_fifteen_blocks_from_an_opening() {
        let registry = BlockRegistry::builtin();
        let generator = WorldGenerator::legacy_flat(TerrainDimensions::new(1, 1, 1, 1), 0);
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Default::default());

        for x in 0..16 {
            for z in 0..16 {
                if x != 0 || z != 0 {
                    world.set_block(IVec3::new(x, 10, z), BlockId::STONE);
                }
            }
        }

        SkylightSystem::rebuild(&mut world, &registry, generator);

        assert_eq!(world.skylight(IVec3::new(0, 9, 0)), 15);
        assert_eq!(world.skylight(IVec3::new(14, 9, 0)), 1);
        assert_eq!(world.skylight(IVec3::new(15, 9, 0)), 0);
    }

    #[test]
    fn sixteen_block_deep_shaft_reaches_darkness() {
        let registry = BlockRegistry::builtin();
        let generator = WorldGenerator::legacy_flat(TerrainDimensions::new(1, 1, 2, 1), 0);
        let mut world = World::default();
        for chunk_y in -2..=0 {
            let position = IVec3::new(0, chunk_y, 0);
            world.insert_chunk(position, generator.generate_chunk(position).unwrap());
        }
        for y in -14..=1 {
            world.set_block(IVec3::new(0, y, 0), BlockId::AIR);
        }

        SkylightSystem::rebuild(&mut world, &registry, generator);

        assert_eq!(world.skylight(IVec3::new(0, 1, 0)), 15);
        assert_eq!(world.skylight(IVec3::new(0, -14, 0)), 0);
    }
}
