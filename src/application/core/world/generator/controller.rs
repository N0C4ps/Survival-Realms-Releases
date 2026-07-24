use glam::{IVec3, UVec3};

use crate::application::core::blocks::BlockId;

use super::{GeneratorKind, TerrainDimensions, legacy_flat, procedural};
use crate::application::core::world::{
    chunk::{CHUNK_SIZE, Chunk, ChunkFlags},
    coordinates,
};

const LEGACY_DIRT_DEPTH: i32 = 4;

#[derive(Clone, Copy, Debug)]
pub struct WorldGenerator {
    dimensions: TerrainDimensions,
    seed: u64,
    kind: GeneratorKind,
}

pub(crate) struct OriginalBlockSampler {
    generator: WorldGenerator,
    caves: Option<procedural::caves::CaveSampler>,
    lakes: Option<procedural::lakes::LakeSampler>,
    underground_lakes: Option<procedural::underground_lakes::UndergroundLakeSampler>,
    sediments: Option<procedural::sediment::SedimentSampler>,
    underground_deposits: Option<procedural::underground_deposits::UndergroundDepositSampler>,
    rivers: Option<procedural::rivers::RiverSampler>,
    big_lakes: Option<procedural::big_lakes::BigLakeSampler>,
    trees: Option<procedural::trees::TreeSampler>,
}

impl OriginalBlockSampler {
    pub fn block(&self, position: IVec3) -> Option<BlockId> {
        self.trees
            .and_then(|trees| trees.block(position, |sample| self.base_block(sample)))
            .or_else(|| self.base_block(position))
    }

    fn base_block(&self, position: IVec3) -> Option<BlockId> {
        let chunk_position = coordinates::split_global(position).0;
        self.generator.contains_chunk(chunk_position).then(|| {
            let surface = self.generator.surface_height(position.x, position.z);
            if position.y > surface && position.y > procedural::continental::SEA_LEVEL {
                return BlockId::AIR;
            }
            let mut block = self.generator.terrain_block(
                position.y,
                surface,
                self.generator.dirt_depth(position.x, position.z),
            );
            let sediment = self
                .sediments
                .map(|sampler| sampler.column(position.x, position.z, surface));
            if let Some(sediment) = sediment {
                block = sediment.terrain_block(position.y, surface, block);
            }
            if let Some(deposits) = self.underground_deposits {
                block = deposits.block(position, surface, block);
            }
            let is_cave = self.caves.as_ref().is_some_and(|caves| {
                let column = caves.column(position.x, position.z, surface);
                caves.is_cave_air(position, column)
                    && (surface >= procedural::continental::SEA_LEVEL
                        || !column.is_ravine(position.y))
            });
            let mut block = if block != BlockId::AIR && !block.is_liquid() && is_cave {
                BlockId::AIR
            } else {
                block
            };
            let big_lake = self
                .big_lakes
                .map(|sampler| sampler.column(position.x, position.z));
            let river = self.rivers.and_then(|sampler| {
                big_lake
                    .is_none_or(|column| {
                        matches!(column, procedural::big_lakes::BigLakeColumn::None)
                    })
                    .then(|| sampler.column(position.x, position.z, surface))
            });
            if let Some(column) = big_lake {
                block = column.block(position.y, surface, block);
                if let Some((sediment, bottom)) = sediment.zip(column.bottom()) {
                    block = sediment.lake_block(position.y, bottom, block);
                }
            }
            if let Some(column) = river {
                block = column.block(position.y, block);
                if let Some((sediment, bottom)) = sediment.zip(column.bottom()) {
                    block = sediment.lake_block(position.y, bottom, block);
                }
            }
            if let Some(lakes) = self.lakes {
                let lake = lakes.column(position.x, position.z);
                block = lake.block(position.y, block);
                if let Some((sediment, bottom)) = sediment.zip(lake.water_bottom()) {
                    block = sediment.lake_block(position.y, bottom, block);
                }
            }
            if let Some(underground_lakes) = self.underground_lakes {
                block = underground_lakes.block(position, surface, block);
            }
            block
        })
    }
}

impl WorldGenerator {
    /// Creates the current generator used by newly-created worlds.
    pub const fn procedural(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV9,
        }
    }

    /// Restores terrain created after rivers/biomes and before trees.
    pub const fn procedural_v8(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV8,
        }
    }

    /// Restores terrain created after underground deposits and before rivers.
    pub const fn procedural_v7(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV7,
        }
    }

    /// Restores terrain created after sediments and before underground deposits.
    pub const fn procedural_v6(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV6,
        }
    }

    /// Restores terrain created after underground lakes and before sediments.
    pub const fn procedural_v5(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV5,
        }
    }

    /// Restores terrain created after oceans and before underground lakes.
    pub const fn procedural_v4(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV4,
        }
    }

    /// Restores terrain from saves created after lakes and before oceans.
    pub const fn procedural_v3(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV3,
        }
    }

    /// Restores terrain from saves created after caves and before lakes.
    pub const fn procedural_v2(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV2,
        }
    }

    /// Restores terrain from saves created before cave generation existed.
    pub const fn procedural_v1(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::ProceduralV1,
        }
    }

    pub const fn legacy_flat(dimensions: TerrainDimensions, seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            kind: GeneratorKind::LegacyFlat,
        }
    }

    pub const fn dimensions(self) -> TerrainDimensions {
        self.dimensions
    }
    pub const fn seed(self) -> u64 {
        self.seed
    }
    pub const fn kind(self) -> GeneratorKind {
        self.kind
    }

    pub fn surface_height(self, x: i32, z: i32) -> i32 {
        match self.kind {
            GeneratorKind::LegacyFlat => legacy_flat::surface_height(x, z),
            GeneratorKind::ProceduralV1
            | GeneratorKind::ProceduralV2
            | GeneratorKind::ProceduralV3 => procedural::surface_height(self.seed, x, z),
            GeneratorKind::ProceduralV4
            | GeneratorKind::ProceduralV5
            | GeneratorKind::ProceduralV6
            | GeneratorKind::ProceduralV7
            | GeneratorKind::ProceduralV8
            | GeneratorKind::ProceduralV9 => {
                procedural::continental::surface_height(self.seed, x, z)
            }
        }
    }

    pub fn dirt_depth(self, x: i32, z: i32) -> i32 {
        match self.kind {
            GeneratorKind::LegacyFlat => LEGACY_DIRT_DEPTH,
            GeneratorKind::ProceduralV1
            | GeneratorKind::ProceduralV2
            | GeneratorKind::ProceduralV3
            | GeneratorKind::ProceduralV4
            | GeneratorKind::ProceduralV5
            | GeneratorKind::ProceduralV6
            | GeneratorKind::ProceduralV7
            | GeneratorKind::ProceduralV8
            | GeneratorKind::ProceduralV9 => procedural::dirt_depth(self.seed, x, z),
        }
    }

    pub fn spawn_block(self) -> IVec3 {
        if !matches!(
            self.kind,
            GeneratorKind::ProceduralV4
                | GeneratorKind::ProceduralV5
                | GeneratorKind::ProceduralV6
                | GeneratorKind::ProceduralV7
                | GeneratorKind::ProceduralV8
                | GeneratorKind::ProceduralV9
        ) {
            return IVec3::new(0, self.surface_height(0, 5), 5);
        }

        let sampler = self.original_block_sampler();
        let origin = glam::IVec2::new(0, 5);
        for radius in 0..=256 {
            let minimum = origin - glam::IVec2::splat(radius);
            let maximum = origin + glam::IVec2::splat(radius);
            for x in minimum.x..=maximum.x {
                for z in [minimum.y, maximum.y] {
                    if let Some(position) = self.valid_spawn_column(&sampler, x, z) {
                        return position;
                    }
                }
            }
            for z in minimum.y + 1..maximum.y {
                for x in [minimum.x, maximum.x] {
                    if let Some(position) = self.valid_spawn_column(&sampler, x, z) {
                        return position;
                    }
                }
            }
        }
        IVec3::new(0, self.surface_height(0, 5), 5)
    }

    pub fn contains_chunk(self, position: IVec3) -> bool {
        let x = centered_chunk_range(self.dimensions.chunks_x());
        let z = centered_chunk_range(self.dimensions.chunks_z());
        let y = -(self.dimensions.chunks_below_sea_level() as i32)
            ..self.dimensions.chunks_above_sea_level() as i32;
        x.contains(&position.x) && y.contains(&position.y) && z.contains(&position.z)
    }

    pub fn generate_chunk(self, position: IVec3) -> Option<Chunk> {
        if !self.contains_chunk(position)
            || position.y * CHUNK_SIZE as i32 > self.maximum_generated_y()
        {
            return None;
        }
        let chunk = self.generate_chunk_data(position);
        (!chunk.is_empty()).then_some(chunk)
    }

    pub fn original_block(self, position: IVec3) -> Option<BlockId> {
        self.original_block_sampler().block(position)
    }

    pub(crate) fn original_block_sampler(self) -> OriginalBlockSampler {
        OriginalBlockSampler {
            generator: self,
            caves: self.cave_sampler(),
            lakes: self.lake_sampler(),
            underground_lakes: self.underground_lake_sampler(),
            sediments: self.sediment_sampler(),
            underground_deposits: self.underground_deposit_sampler(),
            rivers: self.river_sampler(),
            big_lakes: self.big_lake_sampler(),
            trees: self.tree_sampler(),
        }
    }

    fn generate_chunk_data(self, position: IVec3) -> Chunk {
        let mut chunk = Chunk::empty();
        let mut has_light_sources = false;
        let mut has_grass = false;
        let mut has_fluid_sources = false;
        let cave_sampler = self.cave_sampler();
        let lake_sampler = self.lake_sampler();
        let underground_lake_sampler = self.underground_lake_sampler();
        let underground_lake =
            underground_lake_sampler.and_then(|sampler| sampler.chunk_lake(position));
        let sediment_sampler = self.sediment_sampler();
        let underground_deposit_sampler = self.underground_deposit_sampler();
        let river_sampler = self.river_sampler();
        let big_lake_sampler = self.big_lake_sampler();
        let big_lakes = big_lake_sampler
            .map(|sampler| sampler.chunk_lakes(position))
            .unwrap_or_default();
        let underground_deposits = underground_deposit_sampler
            .map(|sampler| sampler.chunk_deposits(position))
            .unwrap_or_default();
        for x in 0..CHUNK_SIZE as u32 {
            for z in 0..CHUNK_SIZE as u32 {
                let column = coordinates::global_from_local(position, UVec3::new(x, 0, z));
                let surface = self.surface_height(column.x, column.z);
                let dirt_depth = self.dirt_depth(column.x, column.z);
                let cave_column = cave_sampler
                    .as_ref()
                    .map(|sampler| sampler.column(column.x, column.z, surface));
                let lake_column = lake_sampler.map(|sampler| sampler.column(column.x, column.z));
                let sediment_column =
                    sediment_sampler.map(|sampler| sampler.column(column.x, column.z, surface));
                let big_lake_column = big_lake_sampler
                    .map(|sampler| sampler.column_from_lakes(column.x, column.z, &big_lakes));
                let river_column = river_sampler.and_then(|sampler| {
                    big_lake_column
                        .is_none_or(|column| {
                            matches!(column, procedural::big_lakes::BigLakeColumn::None)
                        })
                        .then(|| sampler.column(column.x, column.z, surface))
                });
                for y in 0..CHUNK_SIZE as u32 {
                    let local = UVec3::new(x, y, z);
                    let global = coordinates::global_from_local(position, local);
                    let mut block = self.terrain_block(global.y, surface, dirt_depth);
                    if let Some(sediment) = sediment_column {
                        block = sediment.terrain_block(global.y, surface, block);
                    }
                    if let Some(deposits) = underground_deposit_sampler {
                        block = deposits.block_from_deposits(
                            global,
                            surface,
                            block,
                            &underground_deposits,
                        );
                    }
                    let ravine_skylight = cave_sampler.as_ref().zip(cave_column).and_then(
                        |(sampler, cave_column)| {
                            let ocean_column = surface < procedural::continental::SEA_LEVEL;
                            let can_carve = !ocean_column || !cave_column.is_ravine(global.y);
                            if block != BlockId::AIR
                                && !block.is_liquid()
                                && can_carve
                                && sampler.is_cave_air(global, cave_column)
                            {
                                block = BlockId::AIR;
                            }
                            (!ocean_column)
                                .then(|| cave_column.ravine_skylight(global.y))
                                .flatten()
                        },
                    );
                    if let Some(column) = big_lake_column {
                        block = column.block(global.y, surface, block);
                        if let Some((sediment, bottom)) = sediment_column.zip(column.bottom()) {
                            block = sediment.lake_block(global.y, bottom, block);
                        }
                    }
                    if let Some(column) = river_column {
                        block = column.block(global.y, block);
                        if let Some((sediment, bottom)) = sediment_column.zip(column.bottom()) {
                            block = sediment.lake_block(global.y, bottom, block);
                        }
                    }
                    if let Some(lake_column) = lake_column {
                        block = lake_column.block(global.y, block);
                        if let Some((sediment, bottom)) =
                            sediment_column.zip(lake_column.water_bottom())
                        {
                            block = sediment.lake_block(global.y, bottom, block);
                        }
                    }
                    if let Some(underground_lakes) = underground_lake_sampler {
                        block = underground_lakes.block_from_lake(
                            global,
                            surface,
                            block,
                            underground_lake,
                        );
                    }
                    chunk.set_block(local, block);
                    has_grass |= block == BlockId::GRASS;
                    has_fluid_sources |= block == BlockId::WATER || block == BlockId::LAVA;
                    let direct_sky_floor = lake_column
                        .map_or(surface, |lake| lake.direct_sky_floor(surface))
                        .min(big_lake_column.map_or(surface, |lake| lake.direct_sky_floor(surface)))
                        .min(river_column.map_or(surface, |river| river.direct_sky_floor(surface)));
                    if global.y >= direct_sky_floor {
                        chunk.set_skylight(local, 15);
                    } else if let Some(level) = ravine_skylight {
                        chunk.set_skylight(local, level);
                        has_light_sources |= level > 1;
                    }
                }
            }
        }
        if let Some(trees) = self.tree_sampler() {
            let base = self.original_block_sampler();
            for tree in trees.trees_affecting_chunk(position, |sample| base.base_block(sample)) {
                let minimum = tree.minimum().max(position * CHUNK_SIZE as i32);
                let maximum = tree
                    .maximum()
                    .min(position * CHUNK_SIZE as i32 + IVec3::splat(CHUNK_SIZE as i32 - 1));
                for x in minimum.x..=maximum.x {
                    for y in minimum.y..=maximum.y {
                        for z in minimum.z..=maximum.z {
                            let global = IVec3::new(x, y, z);
                            let Some(tree_block) = tree.block_at(global) else {
                                continue;
                            };
                            let local = coordinates::split_global(global).1;
                            if tree_block == BlockId::LEAVES
                                && chunk.block(local) == BlockId::WOOD_LOG
                            {
                                continue;
                            }
                            chunk.set_block(local, tree_block);
                            if tree_block == BlockId::WOOD_LOG
                                && global.y < tree.ground().y + tree.trunk_height()
                            {
                                chunk.set_skylight(local, 0);
                            }
                        }
                    }
                }
            }
        }
        chunk
            .flags_mut()
            .insert(ChunkFlags::LOADED | ChunkFlags::DIRTY);
        if has_light_sources {
            chunk.flags_mut().insert(ChunkFlags::LIGHT_SOURCES);
        }
        if has_grass {
            chunk.flags_mut().insert(ChunkFlags::GRASS_TICKS);
        }
        if has_fluid_sources {
            chunk.flags_mut().insert(ChunkFlags::FLUID_SOURCES);
            chunk.rebuild_fluid_source_candidates();
        }
        chunk
    }

    fn cave_sampler(self) -> Option<procedural::caves::CaveSampler> {
        matches!(
            self.kind,
            GeneratorKind::ProceduralV2
                | GeneratorKind::ProceduralV3
                | GeneratorKind::ProceduralV4
                | GeneratorKind::ProceduralV5
                | GeneratorKind::ProceduralV6
                | GeneratorKind::ProceduralV7
                | GeneratorKind::ProceduralV8
                | GeneratorKind::ProceduralV9
        )
        .then(|| {
            let world_floor =
                -(self.dimensions.chunks_below_sea_level() as i32) * CHUNK_SIZE as i32;
            procedural::caves::CaveSampler::new(self.seed, world_floor)
        })
    }

    fn lake_sampler(self) -> Option<procedural::lakes::LakeSampler> {
        match self.kind {
            GeneratorKind::ProceduralV3 => Some(procedural::lakes::LakeSampler::new(self.seed)),
            GeneratorKind::ProceduralV4 => {
                Some(procedural::lakes::LakeSampler::continental(self.seed))
            }
            GeneratorKind::ProceduralV5 => {
                Some(procedural::lakes::LakeSampler::continental(self.seed))
            }
            GeneratorKind::ProceduralV6 => {
                Some(procedural::lakes::LakeSampler::continental(self.seed))
            }
            GeneratorKind::ProceduralV7 => {
                Some(procedural::lakes::LakeSampler::continental(self.seed))
            }
            GeneratorKind::ProceduralV8 => {
                Some(procedural::lakes::LakeSampler::continental(self.seed))
            }
            GeneratorKind::ProceduralV9 => {
                Some(procedural::lakes::LakeSampler::continental(self.seed))
            }
            _ => None,
        }
    }

    fn underground_lake_sampler(
        self,
    ) -> Option<procedural::underground_lakes::UndergroundLakeSampler> {
        matches!(
            self.kind,
            GeneratorKind::ProceduralV5
                | GeneratorKind::ProceduralV6
                | GeneratorKind::ProceduralV7
                | GeneratorKind::ProceduralV8
                | GeneratorKind::ProceduralV9
        )
        .then(|| {
            let world_floor =
                -(self.dimensions.chunks_below_sea_level() as i32) * CHUNK_SIZE as i32;
            procedural::underground_lakes::UndergroundLakeSampler::new(self.seed, world_floor)
        })
    }

    fn sediment_sampler(self) -> Option<procedural::sediment::SedimentSampler> {
        matches!(
            self.kind,
            GeneratorKind::ProceduralV6
                | GeneratorKind::ProceduralV7
                | GeneratorKind::ProceduralV8
                | GeneratorKind::ProceduralV9
        )
        .then(|| procedural::sediment::SedimentSampler::new(self.seed))
    }

    fn underground_deposit_sampler(
        self,
    ) -> Option<procedural::underground_deposits::UndergroundDepositSampler> {
        matches!(
            self.kind,
            GeneratorKind::ProceduralV7 | GeneratorKind::ProceduralV8 | GeneratorKind::ProceduralV9
        )
        .then(|| procedural::underground_deposits::UndergroundDepositSampler::new(self.seed))
    }

    fn river_sampler(self) -> Option<procedural::rivers::RiverSampler> {
        matches!(
            self.kind,
            GeneratorKind::ProceduralV8 | GeneratorKind::ProceduralV9
        )
        .then(|| procedural::rivers::RiverSampler::new(self.seed))
    }

    fn big_lake_sampler(self) -> Option<procedural::big_lakes::BigLakeSampler> {
        matches!(
            self.kind,
            GeneratorKind::ProceduralV8 | GeneratorKind::ProceduralV9
        )
        .then(|| procedural::big_lakes::BigLakeSampler::new(self.seed))
    }

    fn tree_sampler(self) -> Option<procedural::trees::TreeSampler> {
        (self.kind == GeneratorKind::ProceduralV9)
            .then(|| procedural::trees::TreeSampler::new(self.seed))
    }

    fn terrain_block(self, y: i32, surface: i32, dirt_depth: i32) -> BlockId {
        if !matches!(
            self.kind,
            GeneratorKind::ProceduralV4
                | GeneratorKind::ProceduralV5
                | GeneratorKind::ProceduralV6
                | GeneratorKind::ProceduralV7
                | GeneratorKind::ProceduralV8
                | GeneratorKind::ProceduralV9
        ) {
            return block_at_height(y, surface, dirt_depth);
        }
        continental_block_at_height(y, surface, dirt_depth)
    }

    const fn maximum_generated_y(self) -> i32 {
        match self.kind {
            GeneratorKind::LegacyFlat => 1,
            GeneratorKind::ProceduralV1
            | GeneratorKind::ProceduralV2
            | GeneratorKind::ProceduralV3 => 7,
            GeneratorKind::ProceduralV4
            | GeneratorKind::ProceduralV5
            | GeneratorKind::ProceduralV6
            | GeneratorKind::ProceduralV7
            | GeneratorKind::ProceduralV8 => 16,
            GeneratorKind::ProceduralV9 => 31,
        }
    }

    fn valid_spawn_column(self, sampler: &OriginalBlockSampler, x: i32, z: i32) -> Option<IVec3> {
        let surface = self.surface_height(x, z);
        let position = IVec3::new(x, surface, z);
        (surface > procedural::continental::SEA_LEVEL
            && sampler.block(position) == Some(BlockId::GRASS)
            && sampler.block(position + IVec3::Y) == Some(BlockId::AIR))
        .then_some(position)
    }
}

impl Default for WorldGenerator {
    fn default() -> Self {
        Self::procedural(TerrainDimensions::default(), 0)
    }
}

fn centered_chunk_range(chunk_count: u32) -> std::ops::Range<i32> {
    let start = -(chunk_count as i32 / 2);
    start..start + chunk_count as i32
}

fn block_at_height(y: i32, surface: i32, dirt_depth: i32) -> BlockId {
    if y > surface {
        BlockId::AIR
    } else if y == surface {
        BlockId::GRASS
    } else if y >= surface - dirt_depth {
        BlockId::DIRT
    } else {
        BlockId::STONE
    }
}

fn continental_block_at_height(y: i32, surface: i32, dirt_depth: i32) -> BlockId {
    use procedural::continental::SEA_LEVEL;

    if y > surface {
        if y <= SEA_LEVEL {
            BlockId::WATER
        } else {
            BlockId::AIR
        }
    } else if surface >= SEA_LEVEL && y == surface {
        BlockId::GRASS
    } else if surface >= -4 && y >= surface - dirt_depth {
        BlockId::DIRT
    } else {
        BlockId::STONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continental_surface_is_seeded_and_contains_land_and_ocean() {
        let a = WorldGenerator::procedural(TerrainDimensions::default(), 42);
        let b = WorldGenerator::procedural(TerrainDimensions::default(), 43);
        let heights_a: Vec<_> = (-768..=768)
            .step_by(16)
            .flat_map(|x| {
                (-768..=768)
                    .step_by(16)
                    .map(move |z| a.surface_height(x, z))
            })
            .collect();
        let heights_b: Vec<_> = (-768..=768)
            .step_by(16)
            .flat_map(|x| {
                (-768..=768)
                    .step_by(16)
                    .map(move |z| b.surface_height(x, z))
            })
            .collect();
        assert_eq!(
            heights_a,
            (-768..=768)
                .step_by(16)
                .flat_map(|x| {
                    (-768..=768)
                        .step_by(16)
                        .map(move |z| a.surface_height(x, z))
                })
                .collect::<Vec<_>>()
        );
        assert_ne!(heights_a, heights_b);
        assert!(heights_a.iter().any(|&height| height < 0));
        assert!(heights_a.iter().any(|&height| height > 0));
    }

    #[test]
    fn grass_follows_each_procedural_column_surface() {
        let generator = WorldGenerator::procedural(TerrainDimensions::default(), 7);
        let columns = (-768..=768)
            .step_by(16)
            .flat_map(|x| (-768..=768).step_by(16).map(move |z| (x, z)))
            .filter(|&(x, z)| {
                let surface = generator.surface_height(x, z);
                generator.original_block(IVec3::new(x, surface, z)) == Some(BlockId::GRASS)
            })
            .take(8)
            .collect::<Vec<_>>();
        assert_eq!(columns.len(), 8);
        for (x, z) in columns {
            let surface = generator.surface_height(x, z);
            assert_eq!(
                generator.original_block(IVec3::new(x, surface, z)),
                Some(BlockId::GRASS)
            );
            assert_eq!(
                generator.original_block(IVec3::new(x, surface + 1, z)),
                Some(BlockId::AIR)
            );
        }
    }

    #[test]
    fn procedural_columns_contain_exactly_three_or_four_dirt_blocks() {
        let generator = WorldGenerator::procedural_v3(TerrainDimensions::default(), 84);
        let mut observed_depths = std::collections::BTreeSet::new();

        for x in -32..32 {
            let z = 11;
            let surface = generator.surface_height(x, z);
            let depth = generator.dirt_depth(x, z);
            observed_depths.insert(depth);
            for offset in 1..=depth {
                assert_eq!(
                    generator.original_block(IVec3::new(x, surface - offset, z)),
                    Some(BlockId::DIRT)
                );
            }
        }

        assert_eq!(observed_depths, [3, 4].into());
    }

    #[test]
    fn legacy_generator_keeps_the_old_flat_height() {
        let generator = WorldGenerator::legacy_flat(TerrainDimensions::default(), 99);
        assert_eq!(generator.surface_height(100, -100), 1);
        assert_eq!(
            generator.original_block(IVec3::new(0, 1, 0)),
            Some(BlockId::GRASS)
        );
    }

    #[test]
    fn procedural_v1_saves_keep_the_cave_free_terrain() {
        let generator = WorldGenerator::procedural_v1(TerrainDimensions::default(), 42);

        for x in -16..16 {
            for z in -16..16 {
                assert_eq!(
                    generator.original_block(IVec3::new(x, -32, z)),
                    Some(BlockId::STONE)
                );
            }
        }
    }

    #[test]
    fn procedural_v3_generates_both_lakes_as_registered_sources() {
        let current = WorldGenerator::procedural_v3(TerrainDimensions::default(), 42);
        let previous = WorldGenerator::procedural_v2(TerrainDimensions::default(), 42);
        let lakes = procedural::lakes::LakeSampler::new(42);

        for block in [BlockId::WATER, BlockId::LAVA] {
            let center = lakes.basin_center(block);
            let surface = current.surface_height(center.x, center.y);
            let position = (surface - 6..=surface)
                .map(|y| IVec3::new(center.x, y, center.y))
                .find(|&position| current.original_block(position) == Some(block))
                .expect("lake center must contain its liquid");
            let (chunk_position, local) = coordinates::split_global(position);
            let chunk = current.generate_chunk(chunk_position).unwrap();

            assert_eq!(chunk.block(local), block);
            assert!(chunk.flags().contains(ChunkFlags::FLUID_SOURCES));
            assert_ne!(previous.original_block(position), Some(block));
        }
    }

    #[test]
    fn cave_generation_is_continuous_across_chunk_borders() {
        let generator = WorldGenerator::procedural(TerrainDimensions::default(), 42);
        let original = generator.original_block_sampler();
        let left_position = IVec3::new(0, -2, 0);
        let right_position = IVec3::new(1, -2, 0);
        let left = generator.generate_chunk(left_position).unwrap();
        let right = generator.generate_chunk(right_position).unwrap();

        for y in 0..CHUNK_SIZE as u32 {
            for z in 0..CHUNK_SIZE as u32 {
                let left_local = UVec3::new((CHUNK_SIZE - 1) as u32, y, z);
                let right_local = UVec3::new(0, y, z);
                let left_global = coordinates::global_from_local(left_position, left_local);
                let right_global = coordinates::global_from_local(right_position, right_local);
                assert_eq!(left.block(left_local), original.block(left_global).unwrap());
                assert_eq!(
                    right.block(right_local),
                    original.block(right_global).unwrap()
                );
            }
        }
    }

    #[test]
    fn cliffs_are_uncommon_and_hills_dominate_the_landscape() {
        let generator = WorldGenerator::procedural_v3(TerrainDimensions::default(), 42);
        let heights =
            (-256..256).flat_map(|x| (-256..256).map(move |z| generator.surface_height(x, z)));
        let (total, ordinary) = heights.fold((0usize, 0usize), |(total, ordinary), height| {
            (
                total + 1,
                ordinary + usize::from((-3..=3).contains(&height)),
            )
        });

        assert!(ordinary * 100 / total >= 90);
        assert!(
            ordinary < total,
            "the sampled world should contain rare cliffs"
        );
    }

    #[test]
    fn ocean_fills_every_column_up_to_sea_level_zero() {
        let generator = WorldGenerator::procedural(TerrainDimensions::default(), 91);
        let (x, z, floor) = (-768..=768)
            .step_by(8)
            .flat_map(|x| {
                (-768..=768)
                    .step_by(8)
                    .map(move |z| (x, z, generator.surface_height(x, z)))
            })
            .find(|&(_, _, height)| height < -2)
            .expect("sample must contain an ocean");

        assert_ne!(
            generator.original_block(IVec3::new(x, floor, z)),
            Some(BlockId::WATER)
        );
        for y in floor + 1..=procedural::continental::SEA_LEVEL {
            assert_eq!(
                generator.original_block(IVec3::new(x, y, z)),
                Some(BlockId::WATER)
            );
        }
        assert_eq!(
            generator.original_block(IVec3::new(x, 1, z)),
            Some(BlockId::AIR)
        );
    }

    #[test]
    fn current_generator_adds_beaches_without_changing_v5_terrain() {
        let current = WorldGenerator::procedural(TerrainDimensions::default(), 42);
        let previous = WorldGenerator::procedural_v5(TerrainDimensions::default(), 42);
        let (x, z, surface, material) = (-768..=768)
            .step_by(3)
            .flat_map(|x| {
                (-768..=768).step_by(11).filter_map(move |z| {
                    let surface = current.surface_height(x, z);
                    let material = current.original_block(IVec3::new(x, surface, z))?;
                    (surface >= 0 && matches!(material, BlockId::SAND | BlockId::CLAY))
                        .then_some((x, z, surface, material))
                })
            })
            .next()
            .expect("sample must contain a beach");

        assert!(matches!(material, BlockId::SAND | BlockId::CLAY));
        assert_ne!(
            previous.original_block(IVec3::new(x, surface, z)),
            Some(material)
        );
        assert!(
            (x - 64..=x + 64)
                .step_by(4)
                .any(|sample_x| (z - 64..=z + 64)
                    .step_by(4)
                    .any(|sample_z| current.surface_height(sample_x, sample_z) < 0)),
            "a beach must be close to ocean water"
        );
    }

    #[test]
    fn ocean_and_water_lake_floors_receive_sediment_materials() {
        let generator = WorldGenerator::procedural(TerrainDimensions::default(), 91);
        let allowed = [
            BlockId::SAND,
            BlockId::GRAVEL,
            BlockId::CLAY,
            BlockId::DIRT,
            BlockId::STONE,
        ];
        let ocean_materials = (-512..=512)
            .step_by(7)
            .flat_map(|x| {
                (-512..=512).step_by(17).filter_map(move |z| {
                    let floor = generator.surface_height(x, z);
                    (floor < 0)
                        .then(|| generator.original_block(IVec3::new(x, floor, z)))
                        .flatten()
                })
            })
            .collect::<std::collections::HashSet<_>>();
        for material in allowed {
            assert!(ocean_materials.contains(&material));
        }

        let lakes = procedural::lakes::LakeSampler::continental(91);
        let center = lakes.basin_center(BlockId::WATER);
        let bottom = lakes
            .column(center.x, center.y)
            .water_bottom()
            .expect("water lake center");
        assert!(
            allowed.contains(
                &generator
                    .original_block(IVec3::new(center.x, bottom, center.y))
                    .expect("lake bottom")
            )
        );
    }

    #[test]
    fn underground_deposits_are_rare_and_can_be_buried_or_cave_exposed() {
        let current = WorldGenerator::procedural(TerrainDimensions::default(), 42);
        let previous = WorldGenerator::procedural_v6(TerrainDimensions::default(), 42);
        let mut stone = 0usize;
        let mut deposits = 0usize;
        let mut buried = false;
        let mut exposed = false;

        'chunks: for chunk_x in -5..=5 {
            for chunk_z in -5..=5 {
                let chunk_position = IVec3::new(chunk_x, -2, chunk_z);
                let Some(chunk) = current.generate_chunk(chunk_position) else {
                    continue;
                };
                for x in 0..CHUNK_SIZE as u32 {
                    for y in 0..CHUNK_SIZE as u32 {
                        for z in 0..CHUNK_SIZE as u32 {
                            let local = UVec3::new(x, y, z);
                            let block = chunk.block(local);
                            stone += usize::from(block == BlockId::STONE);
                            if !matches!(block, BlockId::CLAY | BlockId::GRAVEL) {
                                continue;
                            }
                            deposits += 1;
                            let global = coordinates::global_from_local(chunk_position, local);
                            assert_eq!(previous.original_block(global), Some(BlockId::STONE));
                            let neighbours = [
                                IVec3::X,
                                IVec3::NEG_X,
                                IVec3::Y,
                                IVec3::NEG_Y,
                                IVec3::Z,
                                IVec3::NEG_Z,
                            ];
                            let touches_air = neighbours.into_iter().any(|direction| {
                                current.original_block(global + direction) == Some(BlockId::AIR)
                            });
                            exposed |= touches_air;
                            buried |= !touches_air;
                            if exposed && buried && deposits * 50 < stone {
                                break 'chunks;
                            }
                        }
                    }
                }
            }
        }

        assert!(deposits > 0);
        assert!(deposits * 50 < stone, "deposits must remain rare");
        assert!(buried, "some deposits must remain completely underground");
        assert!(exposed, "caves must be able to expose deposit faces");
    }

    #[test]
    fn current_generator_integrates_rivers_and_big_lakes_as_water_sources() {
        let current = WorldGenerator::procedural(TerrainDimensions::default(), 42);
        let previous = WorldGenerator::procedural_v7(TerrainDimensions::default(), 42);
        let rivers = procedural::rivers::RiverSampler::new(42);
        let big_lakes = procedural::big_lakes::BigLakeSampler::new(42);

        let river_water = (-768..=768)
            .step_by(2)
            .flat_map(|x| {
                (-768..=768).step_by(7).filter_map(move |z| {
                    let surface = current.surface_height(x, z);
                    let column = rivers.column(x, z, surface);
                    let bottom = column.bottom()?;
                    if !matches!(
                        big_lakes.column(x, z),
                        procedural::big_lakes::BigLakeColumn::None
                    ) {
                        return None;
                    }
                    let position = IVec3::new(x, bottom + 1, z);
                    (current.original_block(position) == Some(BlockId::WATER)
                        && previous.original_block(position) != Some(BlockId::WATER))
                    .then_some(position)
                })
            })
            .next()
            .expect("current terrain must contain a generated river");

        let center = big_lakes.first_center();
        let big_lake_water = (center.x - 58..=center.x + 58)
            .flat_map(|x| {
                (center.y - 58..=center.y + 58).filter_map(move |z| {
                    let bottom = big_lakes.column(x, z).bottom()?;
                    let position = IVec3::new(x, bottom + 1, z);
                    (current.original_block(position) == Some(BlockId::WATER)
                        && previous.original_block(position) != Some(BlockId::WATER))
                    .then_some(position)
                })
            })
            .next()
            .expect("current terrain must contain a generated big lake");

        for position in [river_water, big_lake_water] {
            let chunk_position = coordinates::split_global(position).0;
            let chunk = current.generate_chunk(chunk_position).expect("water chunk");
            assert!(chunk.flags().contains(ChunkFlags::FLUID_SOURCES));
        }
    }

    #[test]
    fn new_players_spawn_on_land_above_the_ocean() {
        let generator = WorldGenerator::procedural(TerrainDimensions::default(), 37);
        let spawn = generator.spawn_block();

        assert!(spawn.y > procedural::continental::SEA_LEVEL);
        assert_eq!(generator.original_block(spawn), Some(BlockId::GRASS));
        assert_eq!(
            generator.original_block(spawn + IVec3::Y),
            Some(BlockId::AIR)
        );
    }

    #[test]
    fn current_generator_places_complete_trees_and_v8_stays_tree_free() {
        let current = WorldGenerator::procedural(TerrainDimensions::default(), 91);
        let previous = WorldGenerator::procedural_v8(TerrainDimensions::default(), 91);
        let base = current.original_block_sampler();
        let trees = current.tree_sampler().expect("current tree sampler");
        let mut generated_tree = None;
        'search: for x in -64..64 {
            for z in -64..64 {
                if let Some(tree) = trees
                    .trees_affecting_chunk(IVec3::new(x, 0, z), |position| {
                        base.base_block(position)
                    })
                    .into_iter()
                    .next()
                {
                    generated_tree = Some(tree);
                    break 'search;
                }
            }
        }
        let tree = generated_tree.expect("sample must contain a valid generated tree");
        let trunk_bottom = tree.ground() + IVec3::Y;
        let trunk_top = tree.ground() + IVec3::Y * tree.trunk_height();

        assert!(matches!(
            current.original_block(tree.ground()),
            Some(BlockId::GRASS) | Some(BlockId::DIRT)
        ));
        for y in trunk_bottom.y..=trunk_top.y {
            assert_eq!(
                current.original_block(IVec3::new(trunk_bottom.x, y, trunk_bottom.z)),
                Some(BlockId::WOOD_LOG)
            );
        }
        assert_eq!(previous.original_block(trunk_bottom), Some(BlockId::AIR));

        let chunk_position = coordinates::split_global(trunk_top).0;
        let local = coordinates::split_global(trunk_top).1;
        let chunk = current.generate_chunk(chunk_position).expect("tree chunk");
        assert_eq!(chunk.block(local), BlockId::WOOD_LOG);

        let mut world = crate::application::core::world::World::default();
        world.insert_chunk(chunk_position, chunk);
        assert_eq!(
            world.edit_block(trunk_top, BlockId::AIR),
            Some(BlockId::WOOD_LOG)
        );
        assert!(
            world
                .persistent_edits(current)
                .contains(&(trunk_top, BlockId::AIR)),
            "breaking a generated tree must remain saved"
        );
    }
}
