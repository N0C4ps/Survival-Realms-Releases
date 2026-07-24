mod decode;
mod encode;
mod header;

pub(super) use decode::decode;
pub(super) use encode::encode;
pub(crate) use header::{HEADER_SIZE, MAGIC};
pub(crate) const SAVE_FORMAT_VERSION: u32 = header::VERSION;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use glam::{IVec3, Vec3};
    use serde::Serialize;

    use super::*;
    use crate::application::core::{
        blocks::BlockId,
        persistence::format::LevelSnapshot,
        world::{GeneratorKind, TerrainDimensions, World, WorldGenerator},
    };

    #[derive(Serialize)]
    struct LegacyLevelSnapshot {
        seed: u64,
        terrain: crate::application::core::persistence::format::SavedTerrain,
        chunks: HashMap<[i32; 3], Vec<crate::application::core::persistence::format::SavedBlock>>,
    }

    #[derive(Serialize)]
    struct PreviousLevelSnapshot {
        seed: u64,
        terrain: crate::application::core::persistence::format::SavedTerrain,
        generator_version: u8,
        chunks: HashMap<[i32; 3], Vec<crate::application::core::persistence::format::SavedBlock>>,
    }

    #[test]
    fn compressed_level_round_trip_restores_builds_and_holes() {
        let generator = WorldGenerator::procedural(TerrainDimensions::new(1, 1, 1, 1), 42);
        let mut original = World::default();
        original.insert_chunk(IVec3::ZERO, generator.generate_chunk(IVec3::ZERO).unwrap());
        let placed = IVec3::new(2, 15, 2);
        let removed = (0..16)
            .rev()
            .map(|y| IVec3::new(3, y, 2))
            .find(|&position| original.block(position) != BlockId::AIR)
            .expect("generated chunk must contain terrain or ocean water");
        original.edit_block(placed, BlockId::COBBLESTONE);
        original.edit_block(removed, BlockId::AIR);

        let saved_position = Vec3::new(32.25, 8.0, 31.75);
        let bytes = encode(&LevelSnapshot::capture(
            &original,
            generator,
            Some(saved_position),
        ))
        .unwrap();
        let decoded = decode(&bytes).unwrap();
        let restored_generator = decoded.generator().unwrap();
        let mut restored = World::default();
        assert_eq!(decoded.restore_into(&mut restored).unwrap(), 2);
        restored.insert_chunk(
            IVec3::ZERO,
            restored_generator.generate_chunk(IVec3::ZERO).unwrap(),
        );

        assert_eq!(restored_generator.seed(), 42);
        assert_eq!(restored_generator.kind(), GeneratorKind::ProceduralV9);
        assert_eq!(restored_generator.dimensions().chunks_x(), 512);
        assert_eq!(restored_generator.dimensions().chunks_z(), 512);
        assert_eq!(restored_generator.dimensions().chunks_below_sea_level(), 32);
        assert_eq!(restored_generator.dimensions().chunks_above_sea_level(), 32);
        assert_eq!(decoded.player_position().unwrap(), Some(saved_position));
        assert_eq!(restored.block(placed), BlockId::COBBLESTONE);
        assert_eq!(restored.block(removed), BlockId::AIR);
    }

    #[test]
    fn version_one_level_files_keep_the_legacy_flat_generator() {
        let legacy = LegacyLevelSnapshot {
            seed: 99,
            terrain: TerrainDimensions::new(1, 1, 1, 1).into(),
            chunks: HashMap::new(),
        };
        let serialized = bincode::serialize(&legacy).unwrap();
        let compressed = lz4_flex::compress_prepend_size(&serialized);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&header::MAGIC);
        bytes.extend_from_slice(&header::LEGACY_VERSION.to_le_bytes());
        bytes.extend_from_slice(&compressed);

        let decoded = decode(&bytes).unwrap();
        let generator = decoded.generator().unwrap();

        assert_eq!(generator.kind(), GeneratorKind::LegacyFlat);
        assert_eq!(generator.surface_height(120, -80), 1);
        assert_eq!(decoded.player_position().unwrap(), None);
    }

    #[test]
    fn version_two_level_files_load_without_a_player_position() {
        let previous = PreviousLevelSnapshot {
            seed: 101,
            terrain: TerrainDimensions::default().into(),
            generator_version: 1,
            chunks: HashMap::new(),
        };
        let serialized = bincode::serialize(&previous).unwrap();
        let compressed = lz4_flex::compress_prepend_size(&serialized);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&header::MAGIC);
        bytes.extend_from_slice(&header::PREVIOUS_VERSION.to_le_bytes());
        bytes.extend_from_slice(&compressed);

        let decoded = decode(&bytes).unwrap();

        let generator = decoded.generator().unwrap();
        assert_eq!(generator.seed(), 101);
        assert_eq!(generator.kind(), GeneratorKind::ProceduralV1);
        assert_eq!(decoded.player_position().unwrap(), None);
    }
}
