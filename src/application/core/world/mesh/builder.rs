use crate::application::core::blocks::BlockRegistry;

use super::{
    super::{World, chunk::ChunkPos},
    ChunkMesh,
    face::FACES,
    greedy,
};

pub fn build_chunk_mesh(
    world: &World,
    registry: &BlockRegistry,
    chunk_position: ChunkPos,
) -> ChunkMesh {
    let mut mesh = ChunkMesh {
        chunk_position,
        vertices: Vec::new(),
        indices: Vec::new(),
        liquid_vertices: Vec::new(),
        liquid_indices: Vec::new(),
    };
    let Some(chunk) = world.chunk(chunk_position) else {
        return mesh;
    };
    if !chunk
        .flags()
        .contains(super::super::chunk::ChunkFlags::LOADED)
        || chunk.is_empty()
    {
        return mesh;
    }

    for (direction, face) in FACES.iter().enumerate() {
        greedy::append_direction(
            &mut mesh,
            world,
            chunk,
            registry,
            chunk_position,
            direction,
            face,
        );
    }

    mesh
}

#[cfg(test)]
mod tests {
    use glam::IVec3;

    use super::*;
    use crate::application::core::{
        blocks::{BlockId, TEXTURES_PER_BLOCK, TextureFace},
        world::{CHUNK_SIZE, FluidState, chunk::Chunk},
    };

    #[test]
    fn merges_adjacent_coplanar_faces() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Chunk::empty());
        world.set_block(IVec3::ZERO, BlockId::STONE);
        world.set_block(IVec3::X, BlockId::STONE);

        let mesh = build_chunk_mesh(&world, &registry, IVec3::ZERO);

        assert_eq!(mesh.indices.len(), 6 * 6);
        assert_eq!(mesh.vertices.len(), 6 * 4);
    }

    #[test]
    fn exposed_face_at_an_unloaded_chunk_border_keeps_its_sunlight() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Chunk::empty());
        let border = IVec3::new(CHUNK_SIZE as i32 - 1, 4, 3);
        world.set_block(border, BlockId::GRASS);
        world.set_skylight(border, 15);

        let mesh = build_chunk_mesh(&world, &registry, IVec3::ZERO);
        let border_face_is_lit = mesh
            .vertices
            .iter()
            .any(|vertex| vertex.position[0] == CHUNK_SIZE as f32 && vertex.skylight == 15);

        assert!(border_face_is_lit);
    }

    #[test]
    fn grass_and_logs_select_textures_per_face() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Chunk::empty());
        world.set_block(IVec3::new(2, 2, 2), BlockId::GRASS);
        world.set_block(IVec3::new(5, 2, 2), BlockId::WOOD_LOG);

        let mesh = build_chunk_mesh(&world, &registry, IVec3::ZERO);
        for (block, x) in [(BlockId::GRASS, 2.0), (BlockId::WOOD_LOG, 5.0)] {
            assert!(mesh.vertices.iter().any(|vertex| {
                vertex.position[0] >= x
                    && vertex.position[0] <= x + 1.0
                    && vertex.normal == [0.0, 1.0, 0.0]
                    && vertex.texture_layer == TextureFace::Top.layer(block)
            }));
            assert!(mesh.vertices.iter().any(|vertex| {
                vertex.position[0] >= x
                    && vertex.position[0] <= x + 1.0
                    && vertex.normal == [0.0, -1.0, 0.0]
                    && vertex.texture_layer == TextureFace::Bottom.layer(block)
            }));
            assert!(mesh.vertices.iter().any(|vertex| {
                vertex.position[0] >= x
                    && vertex.position[0] <= x + 1.0
                    && vertex.normal[1] == 0.0
                    && vertex.texture_layer == TextureFace::Side.layer(block)
            }));
        }
    }

    #[test]
    fn liquid_geometry_is_separate_from_opaque_geometry() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Chunk::empty());
        world.set_block(IVec3::ZERO, BlockId::STONE);
        world.set_block(IVec3::X, BlockId::WATER);

        let mesh = build_chunk_mesh(&world, &registry, IVec3::ZERO);

        assert!(mesh.has_opaque_geometry());
        assert!(mesh.has_liquid_geometry());
        assert!(mesh.liquid_vertices.iter().all(|vertex| {
            vertex.texture_layer / TEXTURES_PER_BLOCK == u32::from(BlockId::WATER.value())
        }));
    }

    #[test]
    fn liquid_levels_change_only_top_vertices_and_smooth_shared_corners() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Chunk::empty());
        let source = IVec3::new(4, 2, 4);
        let low_flow = source + IVec3::X;
        world.set_block(source, BlockId::WATER);
        world.set_block(low_flow, BlockId::WATER);
        world.set_fluid_state(source, FluidState::SOURCE);
        world.set_fluid_state(low_flow, FluidState::new(8, false));

        let mesh = build_chunk_mesh(&world, &registry, IVec3::ZERO);
        let top_heights: Vec<_> = mesh
            .liquid_vertices
            .iter()
            .filter(|vertex| vertex.normal == [0.0, 1.0, 0.0])
            .map(|vertex| vertex.position[1] - source.y as f32)
            .collect();

        assert!(
            top_heights
                .iter()
                .any(|height| (*height - 0.92).abs() < 0.001)
        );
        assert!(
            top_heights
                .iter()
                .any(|height| (*height - 0.125).abs() < 0.001)
        );
        assert!(
            top_heights
                .iter()
                .any(|height| *height > 0.125 && *height < 0.92)
        );
        assert!(
            mesh.liquid_vertices
                .iter()
                .any(|vertex| vertex.position[1] == source.y as f32)
        );
    }

    #[test]
    fn falling_liquid_is_visually_full_and_lava_uses_its_own_levels() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Chunk::empty());
        let falling_water = IVec3::new(3, 2, 3);
        let low_lava = IVec3::new(8, 2, 8);
        world.set_block(falling_water, BlockId::WATER);
        world.set_block(low_lava, BlockId::LAVA);
        world.set_fluid_state(falling_water, FluidState::new(0, true));
        world.set_fluid_state(low_lava, FluidState::new(5, false));

        let mesh = build_chunk_mesh(&world, &registry, IVec3::ZERO);
        let water_top = mesh
            .liquid_vertices
            .iter()
            .filter(|vertex| {
                vertex.texture_layer / TEXTURES_PER_BLOCK == u32::from(BlockId::WATER.value())
                    && vertex.normal == [0.0, 1.0, 0.0]
            })
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let lava_top = mesh
            .liquid_vertices
            .iter()
            .filter(|vertex| {
                vertex.texture_layer / TEXTURES_PER_BLOCK == u32::from(BlockId::LAVA.value())
                    && vertex.normal == [0.0, 1.0, 0.0]
            })
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);

        assert!((water_top - (falling_water.y + 1) as f32).abs() < 0.001);
        assert!((lava_top - (low_lava.y as f32 + 0.125)).abs() < 0.001);
    }

    #[test]
    fn partial_liquid_keeps_its_top_visible_below_a_solid_ceiling() {
        let registry = BlockRegistry::builtin();
        let mut world = World::default();
        world.insert_chunk(IVec3::ZERO, Chunk::empty());
        let water = IVec3::new(4, 2, 4);
        world.set_block(water, BlockId::WATER);
        world.set_block(water + IVec3::Y, BlockId::DIRT);
        world.set_fluid_state(water, FluidState::SOURCE);

        let mesh = build_chunk_mesh(&world, &registry, IVec3::ZERO);

        assert!(mesh.liquid_vertices.iter().any(|vertex| {
            vertex.texture_layer / TEXTURES_PER_BLOCK == u32::from(BlockId::WATER.value())
                && vertex.normal == [0.0, 1.0, 0.0]
                && vertex.position[1] < (water.y + 1) as f32
        }));
    }
}
