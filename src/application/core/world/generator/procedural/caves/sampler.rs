use glam::IVec3;
use noise::{NoiseFn, Perlin};

use super::{
    CaveColumn,
    config::{
        CONNECTED_HORIZONTAL_FREQUENCY, CONNECTED_SURFACE_CLEARANCE, CONNECTED_THRESHOLD,
        CONNECTED_VERTICAL_FREQUENCY, ISOLATED_FREQUENCY, ISOLATED_MIN_DEPTH, ISOLATED_THRESHOLD,
        RAVINE_EXTRA_DEPTH, RAVINE_LINE_FREQUENCY, RAVINE_MIN_DEPTH, RAVINE_REGION_FREQUENCY,
        RAVINE_REGION_THRESHOLD, SPAWN_RAVINE_CLEAR_RADIUS, WORLD_FLOOR_MARGIN,
    },
};

const CONNECTED_A_SEED: u64 = 0xA24B_AED4_963E_E407;
const CONNECTED_B_SEED: u64 = 0x9FB2_1C65_1E98_DF25;
const ISOLATED_SEED: u64 = 0xC13F_A9A9_02A6_328F;
const ISOLATED_DETAIL_SEED: u64 = 0x91E1_0DA5_C79E_7B1D;
const RAVINE_LINE_SEED: u64 = 0xD1B5_4A32_D192_ED03;
const RAVINE_REGION_SEED: u64 = 0xABC9_8388_FB8F_AC03;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CaveType {
    Connected,
    Isolated,
    Ravine,
}

pub struct CaveSampler {
    connected_a: Perlin,
    connected_b: Perlin,
    isolated: Perlin,
    isolated_detail: Perlin,
    ravine_line: Perlin,
    ravine_region: Perlin,
    world_floor: i32,
}

impl CaveSampler {
    pub fn new(seed: u64, world_floor: i32) -> Self {
        Self {
            connected_a: Perlin::new(noise_seed(seed ^ CONNECTED_A_SEED)),
            connected_b: Perlin::new(noise_seed(seed ^ CONNECTED_B_SEED)),
            isolated: Perlin::new(noise_seed(seed ^ ISOLATED_SEED)),
            isolated_detail: Perlin::new(noise_seed(seed ^ ISOLATED_DETAIL_SEED)),
            ravine_line: Perlin::new(noise_seed(seed ^ RAVINE_LINE_SEED)),
            ravine_region: Perlin::new(noise_seed(seed ^ RAVINE_REGION_SEED)),
            world_floor,
        }
    }

    pub fn column(&self, x: i32, z: i32, surface: i32) -> CaveColumn {
        if x.abs() <= SPAWN_RAVINE_CLEAR_RADIUS && z.abs() <= SPAWN_RAVINE_CLEAR_RADIUS {
            return CaveColumn::new(surface, self.world_floor, f64::MAX, 0);
        }

        let region = self.ravine_region.get([
            x as f64 * RAVINE_REGION_FREQUENCY,
            z as f64 * RAVINE_REGION_FREQUENCY,
        ]);
        if region <= RAVINE_REGION_THRESHOLD {
            return CaveColumn::new(surface, self.world_floor, f64::MAX, 0);
        }

        let line_distance = self
            .ravine_line
            .get([
                x as f64 * RAVINE_LINE_FREQUENCY,
                z as f64 * RAVINE_LINE_FREQUENCY,
            ])
            .abs();
        let intensity =
            ((region - RAVINE_REGION_THRESHOLD) / (1.0 - RAVINE_REGION_THRESHOLD)).clamp(0.0, 1.0);
        let depth = RAVINE_MIN_DEPTH + (intensity * RAVINE_EXTRA_DEPTH as f64).round() as i32;
        CaveColumn::new(surface, self.world_floor, line_distance, depth)
    }

    pub fn cave_type(&self, position: IVec3, column: CaveColumn) -> Option<CaveType> {
        if position.y <= self.world_floor + WORLD_FLOOR_MARGIN || position.y > column.surface() {
            return None;
        }
        if column.is_ravine(position.y) {
            return Some(CaveType::Ravine);
        }

        let depth = column.surface() - position.y;
        if depth < CONNECTED_SURFACE_CLEARANCE {
            return None;
        }

        // Two independent zero-isosurfaces intersect as winding tubes. This gives
        // long connected passages without evaluating several octaves of 3D FBM.
        let point = [
            position.x as f64 * CONNECTED_HORIZONTAL_FREQUENCY,
            position.y as f64 * CONNECTED_VERTICAL_FREQUENCY,
            position.z as f64 * CONNECTED_HORIZONTAL_FREQUENCY,
        ];
        let connected_a = self.connected_a.get(point).abs();
        let connected_b = self.connected_b.get(point).abs();
        if connected_a < CONNECTED_THRESHOLD && connected_b < CONNECTED_THRESHOLD {
            return Some(CaveType::Connected);
        }

        // Deep high-density lobes form closed chambers. The detail gate breaks up
        // large masses and keeps their total volume bounded.
        if depth >= ISOLATED_MIN_DEPTH
            && connected_a > CONNECTED_THRESHOLD * 2.0
            && connected_b > CONNECTED_THRESHOLD * 2.0
        {
            let isolated_point = [
                position.x as f64 * ISOLATED_FREQUENCY,
                position.y as f64 * ISOLATED_FREQUENCY * 0.78,
                position.z as f64 * ISOLATED_FREQUENCY,
            ];
            if self.isolated.get(isolated_point) > ISOLATED_THRESHOLD
                && self.isolated_detail.get([
                    isolated_point[0] * 1.9,
                    isolated_point[1] * 1.9,
                    isolated_point[2] * 1.9,
                ]) > -0.12
            {
                return Some(CaveType::Isolated);
            }
        }

        None
    }

    pub fn is_cave_air(&self, position: IVec3, column: CaveColumn) -> bool {
        self.cave_type(position, column).is_some()
    }
}

fn noise_seed(seed: u64) -> u32 {
    let mut mixed = seed;
    mixed ^= mixed >> 30;
    mixed = mixed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed ^= mixed >> 27;
    mixed = mixed.wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^= mixed >> 31;
    (mixed ^ (mixed >> 32)) as u32
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn all_cave_families_are_present_and_deterministic() {
        let sampler = CaveSampler::new(42, -128);
        let mut found = BTreeSet::new();
        let mut cave_blocks = 0usize;
        let mut sampled_blocks = 0usize;

        for x in (-96..96).step_by(2) {
            for z in (-96..96).step_by(2) {
                let column = sampler.column(x, z, 0);
                for y in (-96..=0).step_by(2) {
                    sampled_blocks += 1;
                    if let Some(kind) = sampler.cave_type(IVec3::new(x, y, z), column) {
                        cave_blocks += 1;
                        found.insert(kind as u8);
                        assert_eq!(sampler.cave_type(IVec3::new(x, y, z), column), Some(kind));
                    }
                }
            }
        }

        assert_eq!(found, [0, 1, 2].into());
        assert!(cave_blocks * 200 >= sampled_blocks);
        assert!(cave_blocks * 4 <= sampled_blocks);
    }

    #[test]
    fn caves_preserve_the_world_floor_and_spawn_surface() {
        let sampler = CaveSampler::new(77, -128);
        for x in -12..=12 {
            for z in -12..=12 {
                let column = sampler.column(x, z, 0);
                assert_eq!(sampler.cave_type(IVec3::new(x, 0, z), column), None);
                assert_eq!(sampler.cave_type(IVec3::new(x, -127, z), column), None);
            }
        }
    }
}
