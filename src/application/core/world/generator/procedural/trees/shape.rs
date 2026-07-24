use glam::IVec3;

use super::config::{CANOPY_RADIUS, MAX_CANOPY_HEIGHT, MAX_TRUNK_HEIGHT, MIN_TRUNK_HEIGHT};
use crate::application::core::blocks::BlockId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TreeShape {
    trunk_height: i32,
    canopy_height: i32,
    silhouette_seed: u64,
}

impl TreeShape {
    pub(super) fn from_identity(identity: u64) -> Self {
        Self {
            trunk_height: MIN_TRUNK_HEIGHT
                + (identity % (MAX_TRUNK_HEIGHT - MIN_TRUNK_HEIGHT + 1) as u64) as i32,
            // One block above the trunk is deliberately much more common.
            canopy_height: if (identity >> 16).is_multiple_of(4) {
                MAX_CANOPY_HEIGHT
            } else {
                1
            },
            silhouette_seed: identity >> 24,
        }
    }

    pub(crate) const fn trunk_height(self) -> i32 {
        self.trunk_height
    }

    pub(super) const fn minimum_offset(self) -> IVec3 {
        IVec3::new(-CANOPY_RADIUS, 1, -CANOPY_RADIUS)
    }

    pub(super) const fn maximum_offset(self) -> IVec3 {
        IVec3::new(
            CANOPY_RADIUS,
            self.trunk_height + self.canopy_height,
            CANOPY_RADIUS,
        )
    }

    pub(super) fn block_at(self, offset: IVec3) -> Option<BlockId> {
        if offset.x == 0 && offset.z == 0 && (1..=self.trunk_height).contains(&offset.y) {
            return Some(BlockId::WOOD_LOG);
        }
        self.is_leaf(offset).then_some(BlockId::LEAVES)
    }

    pub(super) fn is_leaf(self, offset: IVec3) -> bool {
        let trunk_top = self.trunk_height;
        let layer = offset.y - trunk_top;
        if !(-1..=self.canopy_height).contains(&layer) {
            return false;
        }

        // A compact four-layer crown: narrow underneath, full through the
        // middle and narrow again at the top. This avoids square shelves while
        // keeping the recognizable rounded canopy from the reference.
        let radius = match layer {
            -1 => 1,
            0 => CANOPY_RADIUS,
            1 if self.canopy_height == 1 => 1,
            1 => CANOPY_RADIUS,
            2 => 1,
            _ => return false,
        };
        if offset.x.abs() > radius || offset.z.abs() > radius {
            return false;
        }

        if radius == 1 {
            // The lower and upper caps are small 3x3 blobs, with one or two
            // deterministic corners removed to avoid a perfect little cube.
            let corner = offset.x.abs() == 1 && offset.z.abs() == 1;
            let corner_hash = self.silhouette_seed
                ^ (offset.x as u32 as u64).wrapping_mul(0x9E37_79B1)
                ^ (offset.z as u32 as u64).wrapping_mul(0x85EB_CA77)
                ^ (offset.y as u32 as u64).wrapping_mul(0xC2B2_AE3D);
            return !corner || !corner_hash.is_multiple_of(2);
        }

        // The two broad layers are a rounded 5x5 instead of a solid square:
        // all far corners disappear and a few rim blocks vary per tree.
        let distance_squared = offset.x * offset.x + offset.z * offset.z;
        if distance_squared > 5 {
            return false;
        }
        let on_outer_rim = distance_squared == 5;
        let rim_hash = self.silhouette_seed
            ^ (offset.x as u32 as u64).wrapping_mul(0x9E37_79B1)
            ^ (offset.z as u32 as u64).wrapping_mul(0x85EB_CA77)
            ^ (offset.y as u32 as u64).wrapping_mul(0xC2B2_AE3D);
        !on_outer_rim || !rim_hash.is_multiple_of(5)
    }

    pub(super) const fn maximum_reach() -> i32 {
        CANOPY_RADIUS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_shapes_stay_inside_the_requested_ranges() {
        for identity in 0..1_000 {
            let shape = TreeShape::from_identity(identity);
            assert!((4..=7).contains(&shape.trunk_height()));
            assert!((1..=2).contains(&shape.canopy_height));
            assert_eq!(shape.maximum_offset().x - shape.minimum_offset().x + 1, 5);
        }
    }

    #[test]
    fn trunk_wins_over_leaves_in_the_crown() {
        let shape = TreeShape::from_identity(42);
        assert_eq!(
            shape.block_at(IVec3::new(0, shape.trunk_height(), 0)),
            Some(BlockId::WOOD_LOG)
        );
        assert_eq!(
            shape.block_at(IVec3::new(1, shape.trunk_height(), 0)),
            Some(BlockId::LEAVES)
        );
    }

    #[test]
    fn crown_has_a_narrow_base_and_top_around_two_full_middle_layers() {
        let shape = TreeShape::from_identity(42);
        let top = shape.trunk_height();
        let count = |layer| {
            (-2..=2)
                .flat_map(|x| (-2..=2).map(move |z| IVec3::new(x, top + layer, z)))
                .filter(|&offset| shape.is_leaf(offset))
                .count()
        };

        assert!(count(-1) < count(0));
        assert!(count(0) >= 17);
        assert!(count(1) >= 17);
        if shape.canopy_height == 2 {
            assert!(count(2) < count(1));
        }
    }
}
