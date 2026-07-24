use glam::IVec3;

use crate::application::core::{blocks::BlockId, world::CHUNK_SIZE};

use super::{
    UndergroundLakeKind,
    config::{
        FLOOR_CLEARANCE, HORIZONTAL_SPACING, LAVA_CHANCE_PERCENT, MAX_HORIZONTAL_RADIUS,
        MAX_LAVA_CENTER_Y, MAX_VERTICAL_RADIUS, MAX_WATER_CENTER_Y, MIN_HORIZONTAL_RADIUS,
        MIN_VERTICAL_RADIUS, SURFACE_CLEARANCE, VERTICAL_SPACING, WATER_CHANCE_PERCENT,
    },
};

const UNDERGROUND_LAKE_SEED: u64 = 0x510E_527F_ADE6_82D1;
const SHAPE_SEED: u64 = 0x9B05_688C_2B3E_6C1F;

#[derive(Clone, Copy)]
pub(crate) struct UndergroundLakeSampler {
    seed: u64,
    world_floor: i32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct UndergroundLake {
    kind: UndergroundLakeKind,
    center: IVec3,
    radius_x: i32,
    radius_y: i32,
    radius_z: i32,
    liquid_level: i32,
}

impl UndergroundLakeSampler {
    pub(crate) const fn new(seed: u64, world_floor: i32) -> Self {
        Self { seed, world_floor }
    }

    pub(crate) fn block(self, position: IVec3, surface: i32, terrain: BlockId) -> BlockId {
        if position.y > MAX_WATER_CENTER_Y + MAX_VERTICAL_RADIUS {
            return terrain;
        }
        let cell = IVec3::new(
            position.x.div_euclid(HORIZONTAL_SPACING),
            position.y.div_euclid(VERTICAL_SPACING),
            position.z.div_euclid(HORIZONTAL_SPACING),
        );
        self.block_from_lake(position, surface, terrain, self.candidate(cell))
    }

    pub(crate) fn chunk_lake(self, chunk_position: IVec3) -> Option<UndergroundLake> {
        let block_minimum = chunk_position * CHUNK_SIZE as i32;
        if block_minimum.y > MAX_WATER_CENTER_Y + MAX_VERTICAL_RADIUS {
            return None;
        }
        let cell = IVec3::new(
            block_minimum.x.div_euclid(HORIZONTAL_SPACING),
            block_minimum.y.div_euclid(VERTICAL_SPACING),
            block_minimum.z.div_euclid(HORIZONTAL_SPACING),
        );
        self.candidate(cell)
    }

    pub(crate) fn block_from_lake(
        self,
        position: IVec3,
        surface: i32,
        terrain: BlockId,
        lake: Option<UndergroundLake>,
    ) -> BlockId {
        let Some(lake) = lake else {
            return terrain;
        };
        if lake.center.y + lake.radius_y + SURFACE_CLEARANCE > surface
            || lake.center.y - lake.radius_y <= self.world_floor + FLOOR_CLEARANCE
        {
            return terrain;
        }
        let offset = position - lake.center;
        if offset.x.abs() > lake.radius_x
            || offset.y.abs() > lake.radius_y
            || offset.z.abs() > lake.radius_z
        {
            return terrain;
        }
        if self.shape_metric(lake, position) > 1.0 {
            return terrain;
        }
        if position.y <= lake.liquid_level {
            lake.kind.block()
        } else {
            BlockId::AIR
        }
    }

    fn candidate(self, cell: IVec3) -> Option<UndergroundLake> {
        let base = hash3(self.seed ^ UNDERGROUND_LAKE_SEED, cell);
        let roll = base % 100;
        let kind = if roll < LAVA_CHANCE_PERCENT {
            UndergroundLakeKind::Lava
        } else if roll < LAVA_CHANCE_PERCENT + WATER_CHANCE_PERCENT {
            UndergroundLakeKind::Water
        } else {
            return None;
        };

        let horizontal_margin = MAX_HORIZONTAL_RADIUS + 2;
        let horizontal_span = HORIZONTAL_SPACING - horizontal_margin * 2;
        let vertical_margin = MAX_VERTICAL_RADIUS + 2;
        let vertical_span = VERTICAL_SPACING - vertical_margin * 2;
        let center = IVec3::new(
            cell.x * HORIZONTAL_SPACING
                + horizontal_margin
                + ((base >> 8) % horizontal_span as u64) as i32,
            cell.y * VERTICAL_SPACING
                + vertical_margin
                + ((base >> 20) % vertical_span as u64) as i32,
            cell.z * HORIZONTAL_SPACING
                + horizontal_margin
                + ((base >> 32) % horizontal_span as u64) as i32,
        );
        let maximum_center_y = match kind {
            UndergroundLakeKind::Water => MAX_WATER_CENTER_Y,
            UndergroundLakeKind::Lava => MAX_LAVA_CENTER_Y,
        };
        if center.y > maximum_center_y {
            return None;
        }

        let horizontal_radius_span = (MAX_HORIZONTAL_RADIUS - MIN_HORIZONTAL_RADIUS + 1) as u64;
        let vertical_radius_span = (MAX_VERTICAL_RADIUS - MIN_VERTICAL_RADIUS + 1) as u64;
        let radius_x = MIN_HORIZONTAL_RADIUS + ((base >> 44) % horizontal_radius_span) as i32;
        let radius_z = MIN_HORIZONTAL_RADIUS + ((base >> 51) % horizontal_radius_span) as i32;
        let radius_y = MIN_VERTICAL_RADIUS + ((base >> 58) % vertical_radius_span) as i32;
        let liquid_level = center.y
            - match kind {
                UndergroundLakeKind::Water => 1,
                UndergroundLakeKind::Lava => 0,
            };

        Some(UndergroundLake {
            kind,
            center,
            radius_x,
            radius_y,
            radius_z,
            liquid_level,
        })
    }

    fn shape_metric(self, lake: UndergroundLake, position: IVec3) -> f32 {
        let offset = position - lake.center;
        let x = offset.x as f32 / lake.radius_x as f32;
        let y = offset.y as f32 / lake.radius_y as f32;
        let z = offset.z as f32 / lake.radius_z as f32;
        let irregularity = signed_unit(hash3(self.seed ^ SHAPE_SEED, position)) * 0.08;
        x * x + y * y + z * z - irregularity
    }

    #[cfg(test)]
    pub(crate) fn sample_center(self, kind: UndergroundLakeKind, surface: i32) -> IVec3 {
        for cell_y in -16..=0 {
            for cell_x in -24..=24 {
                for cell_z in -24..=24 {
                    let Some(lake) = self.candidate(IVec3::new(cell_x, cell_y, cell_z)) else {
                        continue;
                    };
                    if lake.kind == kind
                        && lake.center.y + lake.radius_y + SURFACE_CLEARANCE <= surface
                        && lake.center.y - lake.radius_y > self.world_floor + FLOOR_CLEARANCE
                    {
                        return lake.center;
                    }
                }
            }
        }
        panic!("expected to find a {kind:?} underground lake");
    }
}

fn hash3(seed: u64, position: IVec3) -> u64 {
    let mut value = seed
        ^ (position.x as u32 as u64).wrapping_mul(0x9E37_79B1_85EB_CA87)
        ^ (position.y as u32 as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
        ^ (position.z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
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
    fn water_and_lava_form_partially_filled_underground_chambers() {
        let sampler = UndergroundLakeSampler::new(42, -512);
        for kind in [UndergroundLakeKind::Water, UndergroundLakeKind::Lava] {
            let center = sampler.sample_center(kind, 0);
            assert_eq!(
                sampler.block(center - IVec3::Y, 0, BlockId::STONE),
                kind.block()
            );
            assert_eq!(
                sampler.block(center + IVec3::Y, 0, BlockId::STONE),
                BlockId::AIR
            );
        }
    }

    #[test]
    fn layout_is_deterministic_and_never_reaches_surface_or_world_floor() {
        let first = UndergroundLakeSampler::new(91, -512);
        let second = UndergroundLakeSampler::new(91, -512);
        for kind in [UndergroundLakeKind::Water, UndergroundLakeKind::Lava] {
            assert_eq!(first.sample_center(kind, 3), second.sample_center(kind, 3));
            let center = first.sample_center(kind, 3);
            assert!(center.y < -SURFACE_CLEARANCE);
            assert!(center.y > -512 + FLOOR_CLEARANCE);
        }
    }

    #[test]
    fn chunk_cached_candidate_matches_individual_block_sampling() {
        let sampler = UndergroundLakeSampler::new(42, -512);
        for kind in [UndergroundLakeKind::Water, UndergroundLakeKind::Lava] {
            let center = sampler.sample_center(kind, 0);
            let chunk = center.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
            let cached = sampler.chunk_lake(chunk);
            for offset in [IVec3::ZERO, IVec3::Y, IVec3::NEG_Y, IVec3::X, IVec3::Z] {
                let position = center + offset;
                assert_eq!(
                    sampler.block(position, 0, BlockId::STONE),
                    sampler.block_from_lake(position, 0, BlockId::STONE, cached)
                );
            }
        }
    }
}
