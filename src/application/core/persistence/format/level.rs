use std::collections::HashMap;

use glam::{IVec3, UVec3, Vec3};
use serde::{Deserialize, Serialize};

use crate::application::core::{
    blocks::BlockId,
    world::{
        CHUNK_SIZE, GENERATOR_VERSION, GeneratorKind, TerrainDimensions, World, WorldGenerator,
        split_global,
    },
};

use super::{block::SavedBlock, terrain::SavedTerrain};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct LevelSnapshot {
    pub(crate) seed: u64,
    pub(crate) terrain: SavedTerrain,
    pub(crate) generator_version: u8,
    pub(crate) chunks: HashMap<[i32; 3], Vec<SavedBlock>>,
    pub(crate) player_position: Option<[f32; 3]>,
}

impl LevelSnapshot {
    pub fn capture(
        world: &World,
        generator: WorldGenerator,
        player_position: Option<Vec3>,
    ) -> Self {
        let mut chunks: HashMap<[i32; 3], Vec<SavedBlock>> = HashMap::new();
        for (global, block) in world.persistent_edits(generator) {
            let (chunk, local) = split_global(global);
            chunks
                .entry(chunk.to_array())
                .or_default()
                .push(SavedBlock {
                    local_index: encode_local(local),
                    block_id: block.value(),
                });
        }
        for blocks in chunks.values_mut() {
            blocks.sort_unstable_by_key(|block| block.local_index);
        }

        Self {
            seed: generator.seed(),
            terrain: generator.dimensions().into(),
            generator_version: match generator.kind() {
                GeneratorKind::LegacyFlat => 0,
                GeneratorKind::ProceduralV1 => 1,
                GeneratorKind::ProceduralV2 => 2,
                GeneratorKind::ProceduralV3 => 3,
                GeneratorKind::ProceduralV4 => 4,
                GeneratorKind::ProceduralV5 => 5,
                GeneratorKind::ProceduralV6 => 6,
                GeneratorKind::ProceduralV7 => 7,
                GeneratorKind::ProceduralV8 => 8,
                GeneratorKind::ProceduralV9 => GENERATOR_VERSION,
            },
            chunks,
            player_position: player_position.map(|position| position.to_array()),
        }
    }

    pub(crate) fn from_legacy(
        seed: u64,
        terrain: SavedTerrain,
        chunks: HashMap<[i32; 3], Vec<SavedBlock>>,
    ) -> Self {
        Self {
            seed,
            terrain,
            generator_version: 0,
            chunks,
            player_position: None,
        }
    }

    pub(crate) fn from_previous(
        seed: u64,
        terrain: SavedTerrain,
        generator_version: u8,
        chunks: HashMap<[i32; 3], Vec<SavedBlock>>,
    ) -> Self {
        Self {
            seed,
            terrain,
            generator_version,
            chunks,
            player_position: None,
        }
    }

    pub fn player_position(&self) -> Result<Option<Vec3>, String> {
        let Some(position) = self.player_position.map(Vec3::from_array) else {
            return Ok(None);
        };
        if !position.is_finite() {
            return Err("level contains an invalid player position".to_owned());
        }
        Ok(Some(position))
    }

    pub fn generator(&self) -> Result<WorldGenerator, String> {
        let dimensions = self.terrain.dimensions()?;
        match self.generator_version {
            0 => Ok(WorldGenerator::legacy_flat(dimensions, self.seed)),
            1 => Ok(WorldGenerator::procedural_v1(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            2 => Ok(WorldGenerator::procedural_v2(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            3 => Ok(WorldGenerator::procedural_v3(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            4 => Ok(WorldGenerator::procedural_v4(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            5 => Ok(WorldGenerator::procedural_v5(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            6 => Ok(WorldGenerator::procedural_v6(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            7 => Ok(WorldGenerator::procedural_v7(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            8 => Ok(WorldGenerator::procedural_v8(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            9 => Ok(WorldGenerator::procedural(
                expanded_dimensions(dimensions),
                self.seed,
            )),
            version => Err(format!("unsupported terrain generator version {version}")),
        }
    }

    pub fn restore_into(&self, world: &mut World) -> Result<usize, String> {
        let mut restored = 0;
        for (&chunk_array, blocks) in &self.chunks {
            let chunk = IVec3::from_array(chunk_array);
            for block in blocks {
                let local = decode_local(block.local_index)?;
                let block_id =
                    BlockId::try_from(block.block_id).map_err(|error| error.to_string())?;
                let global = chunk * CHUNK_SIZE as i32 + local.as_ivec3();
                world.restore_edit(global, block_id);
                restored += 1;
            }
        }
        Ok(restored)
    }
}

fn expanded_dimensions(dimensions: TerrainDimensions) -> TerrainDimensions {
    let defaults = TerrainDimensions::default();
    TerrainDimensions::new(
        dimensions.chunks_x().max(defaults.chunks_x()),
        dimensions.chunks_z().max(defaults.chunks_z()),
        dimensions
            .chunks_below_sea_level()
            .max(defaults.chunks_below_sea_level()),
        dimensions
            .chunks_above_sea_level()
            .max(defaults.chunks_above_sea_level()),
    )
}

fn encode_local(local: UVec3) -> u16 {
    (local.x as usize * CHUNK_SIZE * CHUNK_SIZE + local.y as usize * CHUNK_SIZE + local.z as usize)
        as u16
}

fn decode_local(index: u16) -> Result<UVec3, String> {
    let index = index as usize;
    if index >= CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE {
        return Err(format!("invalid local block index in level: {index}"));
    }
    let x = index / (CHUNK_SIZE * CHUNK_SIZE);
    let remainder = index % (CHUNK_SIZE * CHUNK_SIZE);
    let y = remainder / CHUNK_SIZE;
    let z = remainder % CHUNK_SIZE;
    Ok(UVec3::new(x as u32, y as u32, z as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_block_indices_round_trip() {
        for x in 0..CHUNK_SIZE as u32 {
            for y in 0..CHUNK_SIZE as u32 {
                for z in 0..CHUNK_SIZE as u32 {
                    let local = UVec3::new(x, y, z);
                    assert_eq!(decode_local(encode_local(local)).unwrap(), local);
                }
            }
        }
    }

    #[test]
    fn generator_version_two_keeps_the_pre_lake_terrain() {
        let snapshot = LevelSnapshot::from_previous(
            42,
            TerrainDimensions::default().into(),
            2,
            HashMap::new(),
        );

        assert_eq!(
            snapshot.generator().unwrap().kind(),
            GeneratorKind::ProceduralV2
        );
    }

    #[test]
    fn generator_version_three_keeps_the_pre_ocean_terrain() {
        let snapshot = LevelSnapshot::from_previous(
            42,
            TerrainDimensions::default().into(),
            3,
            HashMap::new(),
        );

        assert_eq!(
            snapshot.generator().unwrap().kind(),
            GeneratorKind::ProceduralV3
        );
    }

    #[test]
    fn generator_version_five_keeps_the_pre_sediment_terrain() {
        let snapshot = LevelSnapshot::from_previous(
            42,
            TerrainDimensions::default().into(),
            5,
            HashMap::new(),
        );

        assert_eq!(
            snapshot.generator().unwrap().kind(),
            GeneratorKind::ProceduralV5
        );
    }

    #[test]
    fn generator_version_six_keeps_the_pre_underground_deposit_terrain() {
        let snapshot = LevelSnapshot::from_previous(
            42,
            TerrainDimensions::default().into(),
            6,
            HashMap::new(),
        );

        assert_eq!(
            snapshot.generator().unwrap().kind(),
            GeneratorKind::ProceduralV6
        );
    }

    #[test]
    fn generator_version_seven_keeps_the_pre_river_terrain() {
        let snapshot = LevelSnapshot::from_previous(
            42,
            TerrainDimensions::default().into(),
            7,
            HashMap::new(),
        );

        assert_eq!(
            snapshot.generator().unwrap().kind(),
            GeneratorKind::ProceduralV7
        );
    }

    #[test]
    fn generator_version_eight_keeps_the_pre_tree_terrain() {
        let snapshot = LevelSnapshot::from_previous(
            42,
            TerrainDimensions::default().into(),
            8,
            HashMap::new(),
        );

        assert_eq!(
            snapshot.generator().unwrap().kind(),
            GeneratorKind::ProceduralV8
        );
    }
}
