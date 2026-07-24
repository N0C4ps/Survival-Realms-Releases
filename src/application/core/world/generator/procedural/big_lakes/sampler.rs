use glam::IVec2;

use crate::application::core::{
    blocks::BlockId,
    world::{CHUNK_SIZE, ChunkPos},
};

use super::config::{
    CHANCE_PERCENT, MAX_RADIUS, MAX_TERRAIN_VARIATION, MIN_RADIUS, SPACING, SPAWN_CLEAR_RADIUS,
};
use crate::application::core::world::generator::procedural::continental::{
    SEA_LEVEL, surface_height,
};

const LAKE_SEED: u64 = 0x0801_F2E2_858E_FC16;
const SHAPE_SEED: u64 = 0x6369_20D8_7157_4E69;
const ISLAND_SEED: u64 = 0xA458_FEA3_F493_3D7E;

#[derive(Clone, Copy)]
pub(crate) struct BigLakeSampler {
    seed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BigLakeColumn {
    None,
    Island,
    Basin { level: i32, bottom: i32 },
}

#[derive(Clone, Copy)]
pub(crate) struct BigLake {
    center: IVec2,
    radii: IVec2,
    level: i32,
    identity: u64,
}

impl BigLakeSampler {
    pub(crate) const fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub(crate) fn column(self, x: i32, z: i32) -> BigLakeColumn {
        let search = MAX_RADIUS + 4;
        let minimum_x = (x - search).div_euclid(SPACING);
        let maximum_x = (x + search).div_euclid(SPACING);
        let minimum_z = (z - search).div_euclid(SPACING);
        let maximum_z = (z + search).div_euclid(SPACING);
        let mut lakes = Vec::new();
        for cell_x in minimum_x..=maximum_x {
            for cell_z in minimum_z..=maximum_z {
                if let Some(lake) = self.candidate(cell_x, cell_z) {
                    lakes.push(lake);
                }
            }
        }
        self.column_from_lakes(x, z, &lakes)
    }

    pub(crate) fn chunk_lakes(self, chunk: ChunkPos) -> Vec<BigLake> {
        let origin = chunk * CHUNK_SIZE as i32;
        let search = MAX_RADIUS + 4;
        let minimum_x = (origin.x - search).div_euclid(SPACING);
        let maximum_x = (origin.x + CHUNK_SIZE as i32 - 1 + search).div_euclid(SPACING);
        let minimum_z = (origin.z - search).div_euclid(SPACING);
        let maximum_z = (origin.z + CHUNK_SIZE as i32 - 1 + search).div_euclid(SPACING);
        let mut lakes = Vec::new();
        for cell_x in minimum_x..=maximum_x {
            for cell_z in minimum_z..=maximum_z {
                if let Some(lake) = self.candidate(cell_x, cell_z) {
                    lakes.push(lake);
                }
            }
        }
        lakes
    }

    pub(crate) fn column_from_lakes(self, x: i32, z: i32, lakes: &[BigLake]) -> BigLakeColumn {
        let position = IVec2::new(x, z);
        let mut best: Option<(BigLake, f32)> = None;
        for &lake in lakes {
            let metric = self.shape_metric(lake, position);
            if metric <= 1.0 && best.is_none_or(|(_, current)| metric < current) {
                best = Some((lake, metric));
            }
        }
        let Some((lake, metric)) = best else {
            return BigLakeColumn::None;
        };
        if self.is_island(lake, position) {
            return BigLakeColumn::Island;
        }
        let depth = 1 + ((1.0 - metric).clamp(0.0, 1.0) * 2.0).round() as i32;
        BigLakeColumn::Basin {
            level: lake.level,
            bottom: lake.level - depth,
        }
    }

    #[cfg(test)]
    pub(crate) fn first_center(self) -> IVec2 {
        for x in -8..=8 {
            for z in -8..=8 {
                if let Some(lake) = self.candidate(x, z) {
                    return lake.center;
                }
            }
        }
        panic!("expected a big lake");
    }

    fn candidate(self, cell_x: i32, cell_z: i32) -> Option<BigLake> {
        let identity = hash(self.seed ^ LAKE_SEED, cell_x, cell_z);
        if identity % 100 >= CHANCE_PERCENT {
            return None;
        }
        let margin = MAX_RADIUS + 5;
        let span = SPACING - margin * 2;
        let center = IVec2::new(
            cell_x * SPACING + margin + ((identity >> 8) % span as u64) as i32,
            cell_z * SPACING + margin + ((identity >> 24) % span as u64) as i32,
        );
        if center.abs().max_element() <= SPAWN_CLEAR_RADIUS {
            return None;
        }
        let radius_span = (MAX_RADIUS - MIN_RADIUS + 1) as u64;
        let radii = IVec2::new(
            MIN_RADIUS + ((identity >> 40) % radius_span) as i32,
            MIN_RADIUS + ((identity >> 48) % radius_span) as i32,
        );
        let samples = [
            center,
            center + IVec2::new(radii.x, 0),
            center - IVec2::new(radii.x, 0),
            center + IVec2::new(0, radii.y),
            center - IVec2::new(0, radii.y),
            center + radii / 2,
            center - radii / 2,
            center + IVec2::new(radii.x / 2, -radii.y / 2),
            center + IVec2::new(-radii.x / 2, radii.y / 2),
        ];
        let (mut minimum, mut maximum) = (i32::MAX, i32::MIN);
        for point in samples {
            let height = surface_height(self.seed, point.x, point.y);
            minimum = minimum.min(height);
            maximum = maximum.max(height);
        }
        if minimum <= SEA_LEVEL + 1 || maximum - minimum > MAX_TERRAIN_VARIATION {
            return None;
        }
        Some(BigLake {
            center,
            radii,
            level: minimum - 1,
            identity,
        })
    }

    fn shape_metric(self, lake: BigLake, position: IVec2) -> f32 {
        let normalized = (position - lake.center).as_vec2() / lake.radii.as_vec2();
        let irregularity = signed_unit(hash(self.seed ^ SHAPE_SEED, position.x, position.y)) * 0.12;
        normalized.length_squared() - irregularity
    }

    fn is_island(self, lake: BigLake, position: IVec2) -> bool {
        let count = 2 + (lake.identity % 4) as i32;
        for index in 0..count {
            let identity = hash(
                self.seed ^ ISLAND_SEED ^ lake.identity,
                index,
                index.rotate_left(1),
            );
            let offset = IVec2::new(
                signed_range(identity >> 8, lake.radii.x * 3 / 5),
                signed_range(identity >> 24, lake.radii.y * 3 / 5),
            );
            let radii = IVec2::new(
                4 + ((identity >> 40) % 7) as i32,
                4 + ((identity >> 48) % 7) as i32,
            );
            let normalized = (position - lake.center - offset).as_vec2() / radii.as_vec2();
            if normalized.length_squared() <= 1.0 {
                return true;
            }
        }
        false
    }
}

impl BigLakeColumn {
    pub(crate) fn block(self, y: i32, surface: i32, terrain: BlockId) -> BlockId {
        match self {
            Self::None | Self::Island => terrain,
            Self::Basin { level, .. } if y > level && y <= surface => BlockId::AIR,
            Self::Basin { level, bottom } if y > bottom && y <= level => BlockId::WATER,
            Self::Basin { bottom, .. } if y == bottom => BlockId::DIRT,
            Self::Basin { .. } => terrain,
        }
    }

    pub(crate) const fn bottom(self) -> Option<i32> {
        match self {
            Self::Basin { bottom, .. } => Some(bottom),
            Self::None | Self::Island => None,
        }
    }

    pub(crate) const fn direct_sky_floor(self, surface: i32) -> i32 {
        match self {
            Self::Basin { bottom, .. } => bottom + 1,
            Self::None | Self::Island => surface,
        }
    }
}

fn signed_range(value: u64, radius: i32) -> i32 {
    (value % (radius as u64 * 2 + 1)) as i32 - radius
}

fn hash(seed: u64, x: i32, z: i32) -> u64 {
    let mut value = seed
        ^ (x as u32 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ (z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn signed_unit(value: u64) -> f32 {
    (value as u32 as f32 / u32::MAX as f32) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_lakes_are_wide_shallow_and_contain_islands() {
        let sampler = BigLakeSampler::new(42);
        let center = sampler.first_center();
        let mut water = 0;
        let mut islands = 0;
        let mut maximum_depth = 0;
        for x in center.x - MAX_RADIUS..=center.x + MAX_RADIUS {
            for z in center.y - MAX_RADIUS..=center.y + MAX_RADIUS {
                match sampler.column(x, z) {
                    BigLakeColumn::Basin { level, bottom } => {
                        water += 1;
                        maximum_depth = maximum_depth.max(level - bottom);
                    }
                    BigLakeColumn::Island => islands += 1,
                    BigLakeColumn::None => {}
                }
            }
        }
        assert!(water > 1_000);
        assert!(islands > 0);
        assert!(maximum_depth <= 3);
    }

    #[test]
    fn chunk_candidate_cache_matches_direct_columns() {
        let sampler = BigLakeSampler::new(91);
        for chunk in [IVec2::new(-8, 5), IVec2::new(12, -3), IVec2::new(0, 9)] {
            let chunk = glam::IVec3::new(chunk.x, 0, chunk.y);
            let lakes = sampler.chunk_lakes(chunk);
            let origin = chunk * CHUNK_SIZE as i32;
            for x in 0..CHUNK_SIZE as i32 {
                for z in 0..CHUNK_SIZE as i32 {
                    assert_eq!(
                        sampler.column_from_lakes(origin.x + x, origin.z + z, &lakes),
                        sampler.column(origin.x + x, origin.z + z)
                    );
                }
            }
        }
    }
}
