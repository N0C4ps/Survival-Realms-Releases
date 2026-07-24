use glam::IVec2;

use crate::application::core::blocks::BlockId;

use super::{
    LakeKind,
    config::{LAVA_RIM_WIDTH, MAX_TERRAIN_VARIATION, SPAWN_CLEAR_RADIUS},
};
use crate::application::core::world::generator::procedural::{
    continental::{SEA_LEVEL, surface_height as continental_surface_height},
    surface_height as legacy_surface_height,
};

const LAKE_SEED: u64 = 0x6A09_E667_F3BC_C909;
const SHAPE_SEED: u64 = 0xBB67_AE85_84CA_A73B;

#[derive(Clone, Copy)]
pub(crate) struct LakeSampler {
    seed: u64,
    continental: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum LakeColumn {
    None,
    Basin {
        kind: LakeKind,
        level: i32,
        bottom: i32,
        terrain_surface: i32,
    },
    LavaRim {
        terrain_surface: i32,
    },
}

#[derive(Clone, Copy, Debug)]
struct Lake {
    kind: LakeKind,
    center: IVec2,
    radius_x: i32,
    radius_z: i32,
    level: i32,
}

impl LakeSampler {
    pub(crate) const fn new(seed: u64) -> Self {
        Self {
            seed,
            continental: false,
        }
    }

    pub(crate) const fn continental(seed: u64) -> Self {
        Self {
            seed,
            continental: true,
        }
    }

    pub(crate) fn column(self, x: i32, z: i32) -> LakeColumn {
        let position = IVec2::new(x, z);
        let mut basin: Option<(Lake, f32)> = None;
        let mut lava_rim: Option<(Lake, f32)> = None;

        for kind in LakeKind::ALL {
            let spacing = kind.spacing();
            let (minimum_radius, maximum_radius) = kind.radius_range();
            let search_radius = maximum_radius + 2;
            let minimum_cell_x = (x - search_radius).div_euclid(spacing);
            let maximum_cell_x = (x + search_radius).div_euclid(spacing);
            let minimum_cell_z = (z - search_radius).div_euclid(spacing);
            let maximum_cell_z = (z + search_radius).div_euclid(spacing);

            for cell_x in minimum_cell_x..=maximum_cell_x {
                for cell_z in minimum_cell_z..=maximum_cell_z {
                    let Some(lake) =
                        self.candidate(kind, cell_x, cell_z, minimum_radius, maximum_radius)
                    else {
                        continue;
                    };
                    let metric = self.shape_metric(lake, position);
                    if metric <= 1.0 && basin.is_none_or(|(_, best_metric)| metric < best_metric) {
                        basin = Some((lake, metric));
                    } else if kind == LakeKind::Lava
                        && metric <= 1.0 + LAVA_RIM_WIDTH
                        && lava_rim.is_none_or(|(_, best_metric)| metric < best_metric)
                    {
                        lava_rim = Some((lake, metric));
                    }
                }
            }
        }

        let terrain_surface = self.surface_height(x, z);
        if let Some((lake, metric)) = basin {
            let center_depth = match lake.kind {
                LakeKind::Water => 3,
                LakeKind::Lava => 2,
            };
            let depth = 1 + ((1.0 - metric).clamp(0.0, 1.0) * center_depth as f32).floor() as i32;
            return LakeColumn::Basin {
                kind: lake.kind,
                level: lake.level,
                bottom: lake.level - depth,
                terrain_surface,
            };
        }
        if lava_rim.is_some() {
            LakeColumn::LavaRim { terrain_surface }
        } else {
            LakeColumn::None
        }
    }

    #[cfg(test)]
    pub(crate) fn basin_center(self, block: BlockId) -> IVec2 {
        let kind = match block {
            BlockId::WATER => LakeKind::Water,
            BlockId::LAVA => LakeKind::Lava,
            _ => panic!("test basin must use a liquid block"),
        };
        let (minimum_radius, maximum_radius) = kind.radius_range();
        for cell_x in -10..=10 {
            for cell_z in -10..=10 {
                let Some(lake) =
                    self.candidate(kind, cell_x, cell_z, minimum_radius, maximum_radius)
                else {
                    continue;
                };
                if self.column(lake.center.x, lake.center.y).kind() == Some(kind) {
                    return lake.center;
                }
            }
        }
        panic!("expected to find a {kind:?} lake");
    }

    fn candidate(
        self,
        kind: LakeKind,
        cell_x: i32,
        cell_z: i32,
        minimum_radius: i32,
        maximum_radius: i32,
    ) -> Option<Lake> {
        let kind_salt = match kind {
            LakeKind::Water => 0x3C6E_F372_FE94_F82B,
            LakeKind::Lava => 0xA54F_F53A_5F1D_36F1,
        };
        let base = hash(self.seed ^ LAKE_SEED ^ kind_salt, cell_x, cell_z);
        if base % 100 >= kind.chance_percent() {
            return None;
        }

        let spacing = kind.spacing();
        let margin = maximum_radius + 2;
        let jitter_span = spacing - margin * 2;
        let center = IVec2::new(
            cell_x * spacing + margin + ((base >> 8) % jitter_span as u64) as i32,
            cell_z * spacing + margin + ((base >> 24) % jitter_span as u64) as i32,
        );
        if center.abs().max_element() <= SPAWN_CLEAR_RADIUS {
            return None;
        }

        let radius_span = (maximum_radius - minimum_radius + 1) as u64;
        let radius_x = minimum_radius + ((base >> 40) % radius_span) as i32;
        let radius_z = minimum_radius + ((base >> 48) % radius_span) as i32;
        let sample_points = [
            center,
            center + IVec2::new(radius_x, 0),
            center - IVec2::new(radius_x, 0),
            center + IVec2::new(0, radius_z),
            center - IVec2::new(0, radius_z),
            center + IVec2::new(radius_x / 2, radius_z / 2),
            center + IVec2::new(radius_x / 2, -radius_z / 2),
            center + IVec2::new(-radius_x / 2, radius_z / 2),
            center + IVec2::new(-radius_x / 2, -radius_z / 2),
        ];
        let mut minimum_surface = i32::MAX;
        let mut maximum_surface = i32::MIN;
        for point in sample_points {
            let height = self.surface_height(point.x, point.y);
            minimum_surface = minimum_surface.min(height);
            maximum_surface = maximum_surface.max(height);
        }
        if maximum_surface - minimum_surface > MAX_TERRAIN_VARIATION {
            return None;
        }
        if self.continental && minimum_surface <= SEA_LEVEL + 1 {
            return None;
        }

        Some(Lake {
            kind,
            center,
            radius_x,
            radius_z,
            level: minimum_surface - 1,
        })
    }

    fn shape_metric(self, lake: Lake, position: IVec2) -> f32 {
        let offset = position - lake.center;
        let x = offset.x as f32 / lake.radius_x as f32;
        let z = offset.y as f32 / lake.radius_z as f32;
        let irregularity = signed_unit(hash(self.seed ^ SHAPE_SEED, position.x, position.y)) * 0.14;
        x * x + z * z - irregularity
    }

    fn surface_height(self, x: i32, z: i32) -> i32 {
        if self.continental {
            continental_surface_height(self.seed, x, z)
        } else {
            legacy_surface_height(self.seed, x, z)
        }
    }
}

impl LakeColumn {
    pub(crate) const fn water_bottom(self) -> Option<i32> {
        match self {
            Self::Basin {
                kind: LakeKind::Water,
                bottom,
                ..
            } => Some(bottom),
            Self::None | Self::Basin { .. } | Self::LavaRim { .. } => None,
        }
    }

    pub(crate) fn block(self, y: i32, terrain: BlockId) -> BlockId {
        match self {
            Self::None => terrain,
            Self::Basin {
                kind,
                level,
                bottom,
                terrain_surface,
            } => {
                if y > level && y <= terrain_surface {
                    BlockId::AIR
                } else if y > bottom && y <= level {
                    kind.block()
                } else if y == bottom || (kind == LakeKind::Lava && y == bottom - 1) {
                    match kind {
                        LakeKind::Water => BlockId::DIRT,
                        LakeKind::Lava => BlockId::STONE,
                    }
                } else {
                    terrain
                }
            }
            Self::LavaRim { terrain_surface }
                if (terrain_surface - 2..=terrain_surface).contains(&y) =>
            {
                BlockId::STONE
            }
            Self::LavaRim { .. } => terrain,
        }
    }

    pub(crate) const fn direct_sky_floor(self, terrain_surface: i32) -> i32 {
        match self {
            Self::Basin { bottom, .. } => bottom + 1,
            Self::None | Self::LavaRim { .. } => terrain_surface,
        }
    }

    #[cfg(test)]
    const fn kind(self) -> Option<LakeKind> {
        match self {
            Self::Basin { kind, .. } => Some(kind),
            Self::None | Self::LavaRim { .. } => None,
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

fn signed_unit(value: u64) -> f32 {
    (value as u32 as f32 / u32::MAX as f32) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    fn find_basin(sampler: LakeSampler, kind: LakeKind) -> IVec3 {
        let block = kind.block();
        let center = sampler.basin_center(block);
        let surface = sampler.surface_height(center.x, center.y);
        IVec3::new(center.x, surface, center.y)
    }

    #[test]
    fn lake_layout_is_deterministic_for_a_seed() {
        let sampler = LakeSampler::new(42);
        for position in [(-77, 31), (19, 94), (215, -108)] {
            let first = sampler.column(position.0, position.1);
            let second = sampler.column(position.0, position.1);
            assert_eq!(
                first.kind(),
                second.kind(),
                "lake kind changed at {position:?}"
            );
        }
    }

    #[test]
    fn both_lake_kinds_exist_and_contain_their_liquid() {
        let sampler = LakeSampler::new(42);
        for (kind, block) in [
            (LakeKind::Water, BlockId::WATER),
            (LakeKind::Lava, BlockId::LAVA),
        ] {
            let position = find_basin(sampler, kind);
            let column = sampler.column(position.x, position.z);
            assert!(
                (-8..=8).any(|offset| column.block(position.y + offset, BlockId::STONE) == block)
            );
        }
    }

    #[test]
    fn lava_lakes_build_a_stone_rim() {
        let sampler = LakeSampler::new(91);
        let lava = find_basin(sampler, LakeKind::Lava);
        let mut found_stone_rim = false;
        for x in lava.x - 12..=lava.x + 12 {
            for z in lava.z - 12..=lava.z + 12 {
                let column = sampler.column(x, z);
                if let LakeColumn::LavaRim { terrain_surface } = column {
                    found_stone_rim |=
                        column.block(terrain_surface, BlockId::GRASS) == BlockId::STONE;
                }
            }
        }
        assert!(found_stone_rim);
    }
}
