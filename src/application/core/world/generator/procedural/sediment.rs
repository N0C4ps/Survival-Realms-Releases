use crate::application::core::blocks::BlockId;

use super::{
    continental::{SEA_LEVEL, continentalness},
    noise::value_noise,
};

const BEACH_WIDTH_SEED: u64 = 0xA409_3822_299F_31D0;
const BEACH_CLAY_SEED: u64 = 0x082E_FA98_EC4E_6C89;
const OCEAN_FLOOR_SEED: u64 = 0x4528_21E6_38D0_1377;
const LAKE_FLOOR_SEED: u64 = 0xBE54_66CF_34E9_0C6C;
const DEPTH_SEED: u64 = 0xC0AC_29B7_C97C_50DD;

#[derive(Clone, Copy)]
pub(crate) struct SedimentSampler {
    seed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SedimentColumn {
    terrain: Option<Deposit>,
    lake_floor: Deposit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Deposit {
    block: BlockId,
    depth: i32,
}

impl SedimentSampler {
    pub(crate) const fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub(crate) fn column(self, x: i32, z: i32, surface: i32) -> SedimentColumn {
        let terrain = if surface < SEA_LEVEL {
            Some(self.deposit(x, z, OCEAN_FLOOR_SEED))
        } else if self.is_beach(x, z, surface) {
            let clay_patch = value_noise(self.seed ^ BEACH_CLAY_SEED, x, z, 13) > 0.60;
            Some(Deposit {
                block: if clay_patch {
                    BlockId::CLAY
                } else {
                    BlockId::SAND
                },
                depth: 2 + (hash(self.seed ^ DEPTH_SEED, x, z) % 3) as i32,
            })
        } else {
            None
        };

        SedimentColumn {
            terrain,
            lake_floor: self.deposit(x, z, LAKE_FLOOR_SEED),
        }
    }

    fn is_beach(self, x: i32, z: i32, surface: i32) -> bool {
        let width_noise = value_noise(self.seed ^ BEACH_WIDTH_SEED, x, z, 48);
        let continental_limit = 0.035 + (width_noise * 0.5 + 0.5) * 0.055;
        let maximum_height = 1 + ((width_noise * 0.5 + 0.5) * 2.0).round() as i32;
        surface <= maximum_height && continentalness(self.seed, x, z) <= continental_limit
    }

    fn deposit(self, x: i32, z: i32, salt: u64) -> Deposit {
        let field = value_noise(self.seed ^ salt, x, z, 15);
        let block = if field < -0.58 {
            BlockId::CLAY
        } else if field < -0.16 {
            BlockId::GRAVEL
        } else if field < 0.38 {
            BlockId::SAND
        } else if field < 0.70 {
            BlockId::DIRT
        } else {
            BlockId::STONE
        };
        let maximum_depth = match block {
            BlockId::SAND | BlockId::DIRT => 3,
            BlockId::CLAY | BlockId::GRAVEL => 2,
            _ => 1,
        };
        Deposit {
            block,
            depth: 1 + (hash(self.seed ^ salt ^ DEPTH_SEED, x, z) % maximum_depth) as i32,
        }
    }
}

impl SedimentColumn {
    pub(crate) fn terrain_block(self, y: i32, surface: i32, block: BlockId) -> BlockId {
        self.terrain
            .map_or(block, |deposit| deposit.apply(y, surface, block))
    }

    pub(crate) fn lake_block(self, y: i32, bottom: i32, block: BlockId) -> BlockId {
        self.lake_floor.apply(y, bottom, block)
    }
}

impl Deposit {
    fn apply(self, y: i32, top: i32, original: BlockId) -> BlockId {
        let inside = y <= top && y > top - self.depth;
        if inside && original != BlockId::AIR && !original.is_liquid() {
            self.block
        } else {
            original
        }
    }
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::application::core::world::generator::procedural::continental::surface_height;

    #[test]
    fn beaches_are_sand_dominant_with_sparse_clay() {
        let sampler = SedimentSampler::new(42);
        let mut sand = 0;
        let mut clay = 0;
        for x in (-512..=512).step_by(4) {
            for z in (-512..=512).step_by(4) {
                let surface = surface_height(42, x, z);
                let column = sampler.column(x, z, surface);
                match column.terrain {
                    Some(Deposit {
                        block: BlockId::SAND,
                        ..
                    }) if surface >= SEA_LEVEL => sand += 1,
                    Some(Deposit {
                        block: BlockId::CLAY,
                        ..
                    }) if surface >= SEA_LEVEL => clay += 1,
                    _ => {}
                }
            }
        }

        assert!(sand > 0);
        assert!(clay > 0);
        assert!(sand > clay);
    }

    #[test]
    fn submerged_floors_contain_every_requested_material() {
        let sampler = SedimentSampler::new(91);
        let materials = (-768..=768)
            .step_by(3)
            .flat_map(|x| {
                (-768..=768).step_by(19).filter_map(move |z| {
                    let surface = surface_height(91, x, z);
                    (surface < SEA_LEVEL)
                        .then(|| sampler.column(x, z, surface).terrain.unwrap().block)
                })
            })
            .collect::<HashSet<_>>();

        for expected in [
            BlockId::SAND,
            BlockId::GRAVEL,
            BlockId::CLAY,
            BlockId::DIRT,
            BlockId::STONE,
        ] {
            assert!(materials.contains(&expected), "missing {expected:?}");
        }
    }
}
