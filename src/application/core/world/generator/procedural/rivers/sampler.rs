use glam::IVec2;

use crate::application::core::blocks::BlockId;

use super::config::{
    MIN_WIDTH_THRESHOLD, PATH_SCALE, REGION_SCALE, SPAWN_CLEAR_RADIUS, WARP_SCALE, WARP_STRENGTH,
    WIDTH_SCALE, WIDTH_VARIATION,
};
use crate::application::core::world::generator::procedural::noise::value_noise;

const PATH_SEED: u64 = 0xD131_0BA6_98DF_B5AC;
const WARP_X_SEED: u64 = 0x2FFD_72DB_D01A_DFB7;
const WARP_Z_SEED: u64 = 0xB8E1_AFED_6A26_7E96;
const REGION_SEED: u64 = 0xBA7C_9045_F12C_7F99;
const WIDTH_SEED: u64 = 0x24A1_9947_B391_6CF7;

#[derive(Clone, Copy)]
pub(crate) struct RiverSampler {
    seed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RiverColumn {
    None,
    Channel { level: i32, bottom: i32 },
}

impl RiverSampler {
    pub(crate) const fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub(crate) fn column(self, x: i32, z: i32, surface: i32) -> RiverColumn {
        if surface <= 0 || IVec2::new(x, z).abs().max_element() <= SPAWN_CLEAR_RADIUS {
            return RiverColumn::None;
        }
        let warp_x = value_noise(self.seed ^ WARP_X_SEED, x, z, WARP_SCALE) * WARP_STRENGTH;
        let warp_z = value_noise(self.seed ^ WARP_Z_SEED, x, z, WARP_SCALE) * WARP_STRENGTH;
        let warped_x = x + warp_x.round() as i32;
        let warped_z = z + warp_z.round() as i32;
        let path = value_noise(self.seed ^ PATH_SEED, warped_x, warped_z, PATH_SCALE).abs();
        if value_noise(self.seed ^ REGION_SEED, x, z, REGION_SCALE) < -0.42 {
            return RiverColumn::None;
        }
        let width_noise = value_noise(self.seed ^ WIDTH_SEED, x, z, WIDTH_SCALE);
        let threshold = MIN_WIDTH_THRESHOLD + (width_noise * 0.5 + 0.5) * WIDTH_VARIATION;
        if path > threshold {
            return RiverColumn::None;
        }
        let strength = 1.0 - (path / threshold).clamp(0.0, 1.0);
        let depth = 1 + (strength * 2.0).round() as i32;
        let level = surface - 1;
        RiverColumn::Channel {
            level,
            bottom: level - depth,
        }
    }
}

impl RiverColumn {
    pub(crate) fn block(self, y: i32, terrain: BlockId) -> BlockId {
        match self {
            Self::None => terrain,
            Self::Channel { level, .. } if y > level => BlockId::AIR,
            Self::Channel { level, bottom } if y > bottom && y <= level => BlockId::WATER,
            Self::Channel { bottom, .. } if y == bottom => BlockId::DIRT,
            Self::Channel { .. } => terrain,
        }
    }

    pub(crate) const fn bottom(self) -> Option<i32> {
        match self {
            Self::Channel { bottom, .. } => Some(bottom),
            Self::None => None,
        }
    }

    pub(crate) const fn direct_sky_floor(self, surface: i32) -> i32 {
        match self {
            Self::Channel { bottom, .. } => bottom + 1,
            Self::None => surface,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::core::world::generator::procedural::continental::surface_height;

    #[test]
    fn rivers_are_deterministic_and_do_not_cover_land_like_oceans() {
        let sampler = RiverSampler::new(42);
        let mut land = 0usize;
        let mut river = 0usize;
        for x in -512..=512 {
            for z in (-512..=512).step_by(4) {
                let surface = surface_height(42, x, z);
                if surface > 0 {
                    land += 1;
                    let column = sampler.column(x, z, surface);
                    river += usize::from(!matches!(column, RiverColumn::None));
                    assert_eq!(column, sampler.column(x, z, surface));
                }
            }
        }
        assert!(river > 0);
        assert!(river * 10 < land);
    }
}
