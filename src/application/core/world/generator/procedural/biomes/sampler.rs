use glam::IVec2;

use super::{
    BiomeZone,
    config::{
        DREAMCORE_CHANCE_PERCENT, DREAMCORE_MAX_RADIUS, DREAMCORE_MIN_RADIUS, DREAMCORE_SPACING,
        FOREST_BROAD_SCALE, FOREST_DETAIL_SCALE, FOREST_THRESHOLD, FOREST_WARP_SCALE,
        FOREST_WARP_STRENGTH,
    },
    hash,
};
use crate::application::core::world::generator::procedural::{
    continental::continentalness, noise::value_noise,
};

const FOREST_SEED: u64 = 0x19A4_C116_B8D2_D0C8;
const FOREST_DETAIL_SEED: u64 = 0x1E37_6C08_5141_AB53;
const WARP_X_SEED: u64 = 0x2748_77A2_F8F7_8DF3;
const WARP_Z_SEED: u64 = 0x34B0_BCB5_E19B_48A8;
const DREAMCORE_SEED: u64 = 0x391C_0CB3_C5C9_5A63;
const DREAMCORE_SHAPE_SEED: u64 = 0x4ED8_AA4A_E341_8ACB;

#[derive(Clone, Copy)]
pub(crate) struct BiomeSampler {
    seed: u64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DreamcoreZone {
    center: IVec2,
    radii: IVec2,
}

impl BiomeSampler {
    pub(crate) const fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub(super) const fn seed(self) -> u64 {
        self.seed
    }

    pub(crate) fn zone_at(self, x: i32, z: i32) -> BiomeZone {
        if self.dreamcore_zone_at(x, z).is_some() {
            return BiomeZone::VeryDreamcoreOneTree;
        }
        let warp_x =
            value_noise(self.seed ^ WARP_X_SEED, x, z, FOREST_WARP_SCALE) * FOREST_WARP_STRENGTH;
        let warp_z =
            value_noise(self.seed ^ WARP_Z_SEED, x, z, FOREST_WARP_SCALE) * FOREST_WARP_STRENGTH;
        let x = x + warp_x.round() as i32;
        let z = z + warp_z.round() as i32;
        let forest = value_noise(self.seed ^ FOREST_SEED, x, z, FOREST_BROAD_SCALE) * 0.74
            + value_noise(self.seed ^ FOREST_DETAIL_SEED, x, z, FOREST_DETAIL_SCALE) * 0.26;
        if forest > FOREST_THRESHOLD {
            BiomeZone::Forest
        } else {
            BiomeZone::Plains
        }
    }

    pub(super) fn dreamcore_zone_at(self, x: i32, z: i32) -> Option<DreamcoreZone> {
        let search = DREAMCORE_MAX_RADIUS + 4;
        let min_x = (x - search).div_euclid(DREAMCORE_SPACING);
        let max_x = (x + search).div_euclid(DREAMCORE_SPACING);
        let min_z = (z - search).div_euclid(DREAMCORE_SPACING);
        let max_z = (z + search).div_euclid(DREAMCORE_SPACING);
        let position = IVec2::new(x, z);
        for cell_x in min_x..=max_x {
            for cell_z in min_z..=max_z {
                let Some(zone) = self.dreamcore_candidate(cell_x, cell_z) else {
                    continue;
                };
                let normalized = (position - zone.center).as_vec2() / zone.radii.as_vec2();
                let irregularity = value_noise(self.seed ^ DREAMCORE_SHAPE_SEED, x, z, 47) * 0.13;
                if normalized.length_squared() <= 1.0 + irregularity {
                    return Some(zone);
                }
            }
        }
        None
    }

    fn dreamcore_candidate(self, cell_x: i32, cell_z: i32) -> Option<DreamcoreZone> {
        let identity = hash(self.seed ^ DREAMCORE_SEED, cell_x, cell_z);
        if identity % 100 >= DREAMCORE_CHANCE_PERCENT {
            return None;
        }
        let margin = DREAMCORE_MAX_RADIUS + 12;
        let span = DREAMCORE_SPACING - margin * 2;
        let center = IVec2::new(
            cell_x * DREAMCORE_SPACING + margin + ((identity >> 8) % span as u64) as i32,
            cell_z * DREAMCORE_SPACING + margin + ((identity >> 24) % span as u64) as i32,
        );
        if continentalness(self.seed, center.x, center.y) < 0.10 {
            return None;
        }
        let radius_span = (DREAMCORE_MAX_RADIUS - DREAMCORE_MIN_RADIUS + 1) as u64;
        Some(DreamcoreZone {
            center,
            radii: IVec2::new(
                DREAMCORE_MIN_RADIUS + ((identity >> 40) % radius_span) as i32,
                DREAMCORE_MIN_RADIUS + ((identity >> 48) % radius_span) as i32,
            ),
        })
    }
}

impl DreamcoreZone {
    pub(super) const fn center(self) -> IVec2 {
        self.center
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_boundaries_are_seeded_and_not_axis_aligned_grids() {
        let sampler = BiomeSampler::new(42);
        let row = (-1_024..=1_024)
            .map(|x| sampler.zone_at(x, 173))
            .collect::<Vec<_>>();
        assert!(row.contains(&BiomeZone::Plains));
        assert!(row.contains(&BiomeZone::Forest));
        assert_eq!(
            row,
            (-1_024..=1_024)
                .map(|x| sampler.zone_at(x, 173))
                .collect::<Vec<_>>()
        );
        assert!(
            row.windows(2)
                .enumerate()
                .any(|(x, pair)| pair[0] != pair[1] && (x as i32 - 1_024) % 16 != 0)
        );
    }

    #[test]
    fn dreamcore_zones_are_very_rare_and_have_one_anchor() {
        let sampler = BiomeSampler::new(7);
        let mut dreamcore = 0usize;
        let mut anchors = std::collections::HashSet::new();
        for x in (-8_192..8_192).step_by(32) {
            for z in (-8_192..8_192).step_by(32) {
                if let Some(zone) = sampler.dreamcore_zone_at(x, z) {
                    dreamcore += 1;
                    anchors.insert(zone.center());
                }
            }
        }
        assert!(dreamcore > 0);
        assert!(dreamcore * 200 < 512 * 512);
        assert!(anchors.len() < 12);
    }
}
