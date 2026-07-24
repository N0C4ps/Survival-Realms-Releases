use glam::IVec3;

use crate::application::core::{
    blocks::BlockId,
    world::{CHUNK_SIZE, ChunkPos},
};

use super::config::{
    CELL_SIZE, CLAY_CHANCE_PER_THOUSAND, GRAVEL_CHANCE_PER_THOUSAND, MAX_RADIUS, SURFACE_CLEARANCE,
};

const DEPOSIT_SEED: u64 = 0x3F84_D5B5_B547_0917;
const SHAPE_SEED: u64 = 0x9216_D5D9_8979_FB1B;

#[derive(Clone, Copy)]
pub(crate) struct UndergroundDepositSampler {
    seed: u64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct UndergroundDeposit {
    center: IVec3,
    radii: IVec3,
    block: BlockId,
}

impl UndergroundDepositSampler {
    pub(crate) const fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub(crate) fn chunk_deposits(self, chunk: ChunkPos) -> Vec<UndergroundDeposit> {
        let minimum = chunk * CHUNK_SIZE as i32 - IVec3::splat(MAX_RADIUS);
        let maximum = minimum + IVec3::splat(CHUNK_SIZE as i32 - 1 + MAX_RADIUS * 2);
        let minimum_cell = minimum.map(|axis| axis.div_euclid(CELL_SIZE));
        let maximum_cell = maximum.map(|axis| axis.div_euclid(CELL_SIZE));
        let mut deposits = Vec::new();

        for cell_x in minimum_cell.x..=maximum_cell.x {
            for cell_y in minimum_cell.y..=maximum_cell.y {
                for cell_z in minimum_cell.z..=maximum_cell.z {
                    if let Some(deposit) = self.candidate(IVec3::new(cell_x, cell_y, cell_z))
                        && deposit.overlaps(minimum, maximum)
                    {
                        deposits.push(deposit);
                    }
                }
            }
        }
        deposits
    }

    pub(crate) fn block_from_deposits(
        self,
        position: IVec3,
        surface: i32,
        original: BlockId,
        deposits: &[UndergroundDeposit],
    ) -> BlockId {
        if original != BlockId::STONE || position.y > surface - SURFACE_CLEARANCE {
            return original;
        }
        deposits
            .iter()
            .find(|deposit| deposit.contains(self.seed, position))
            .map_or(original, |deposit| deposit.block)
    }

    pub(crate) fn block(self, position: IVec3, surface: i32, original: BlockId) -> BlockId {
        if original != BlockId::STONE || position.y > surface - SURFACE_CLEARANCE {
            return original;
        }
        let cell = position.map(|axis| axis.div_euclid(CELL_SIZE));
        for x in cell.x - 1..=cell.x + 1 {
            for y in cell.y - 1..=cell.y + 1 {
                for z in cell.z - 1..=cell.z + 1 {
                    if let Some(deposit) = self.candidate(IVec3::new(x, y, z))
                        && deposit.contains(self.seed, position)
                    {
                        return deposit.block;
                    }
                }
            }
        }
        original
    }

    fn candidate(self, cell: IVec3) -> Option<UndergroundDeposit> {
        let value = hash3(self.seed ^ DEPOSIT_SEED, cell);
        let roll = value % 1_000;
        let block = if roll < GRAVEL_CHANCE_PER_THOUSAND {
            BlockId::GRAVEL
        } else if roll < GRAVEL_CHANCE_PER_THOUSAND + CLAY_CHANCE_PER_THOUSAND {
            BlockId::CLAY
        } else {
            return None;
        };
        let margin = MAX_RADIUS;
        let jitter_span = CELL_SIZE - margin * 2;
        let center = cell * CELL_SIZE
            + IVec3::new(
                margin + ((value >> 10) % jitter_span as u64) as i32,
                margin + ((value >> 20) % jitter_span as u64) as i32,
                margin + ((value >> 30) % jitter_span as u64) as i32,
            );
        let radii = IVec3::new(
            2 + ((value >> 40) % 3) as i32,
            1 + ((value >> 44) % 3) as i32,
            2 + ((value >> 48) % 3) as i32,
        );
        Some(UndergroundDeposit {
            center,
            radii,
            block,
        })
    }
}

impl UndergroundDeposit {
    fn contains(self, seed: u64, position: IVec3) -> bool {
        let offset = position - self.center;
        if offset.abs().cmple(self.radii).all() {
            let normalized = offset.as_vec3() / self.radii.as_vec3();
            let irregularity = signed_unit(hash3(seed ^ SHAPE_SEED, position)) * 0.14;
            normalized.length_squared() <= 1.0 + irregularity
        } else {
            false
        }
    }

    fn overlaps(self, minimum: IVec3, maximum: IVec3) -> bool {
        let deposit_minimum = self.center - self.radii;
        let deposit_maximum = self.center + self.radii;
        deposit_minimum.cmple(maximum).all() && deposit_maximum.cmpge(minimum).all()
    }
}

fn hash3(seed: u64, position: IVec3) -> u64 {
    let mut value = seed
        ^ (position.x as u32 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ (position.y as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (position.z as u32 as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
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
    fn deposits_are_deterministic_sparse_and_contain_both_materials() {
        let sampler = UndergroundDepositSampler::new(42);
        let mut gravel = 0;
        let mut clay = 0;
        let mut total = 0;
        for x in -8..=8 {
            for y in -8..=0 {
                for z in -8..=8 {
                    total += 1;
                    if let Some(deposit) = sampler.candidate(IVec3::new(x, y, z)) {
                        assert_eq!(
                            sampler.candidate(IVec3::new(x, y, z)).unwrap().block,
                            deposit.block
                        );
                        gravel += usize::from(deposit.block == BlockId::GRAVEL);
                        clay += usize::from(deposit.block == BlockId::CLAY);
                    }
                }
            }
        }

        assert!(gravel > clay && clay > 0);
        assert!((gravel + clay) * 5 < total);
    }

    #[test]
    fn chunk_cache_matches_direct_sampling() {
        let sampler = UndergroundDepositSampler::new(91);
        let chunk = IVec3::new(2, -3, -1);
        let deposits = sampler.chunk_deposits(chunk);
        let origin = chunk * CHUNK_SIZE as i32;
        for x in 0..CHUNK_SIZE as i32 {
            for y in 0..CHUNK_SIZE as i32 {
                for z in 0..CHUNK_SIZE as i32 {
                    let position = origin + IVec3::new(x, y, z);
                    assert_eq!(
                        sampler.block_from_deposits(position, 4, BlockId::STONE, &deposits),
                        sampler.block(position, 4, BlockId::STONE)
                    );
                }
            }
        }
    }
}
