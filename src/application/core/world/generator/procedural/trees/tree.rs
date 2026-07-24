use glam::{IVec2, IVec3};

use super::shape::TreeShape;
use crate::application::core::blocks::BlockId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedTree {
    ground: IVec3,
    shape: TreeShape,
}

impl GeneratedTree {
    pub(super) const fn new(horizontal: IVec2, ground_y: i32, shape: TreeShape) -> Self {
        Self {
            ground: IVec3::new(horizontal.x, ground_y, horizontal.y),
            shape,
        }
    }

    pub(crate) const fn ground(self) -> IVec3 {
        self.ground
    }

    pub(crate) const fn trunk_height(self) -> i32 {
        self.shape.trunk_height()
    }

    pub(crate) fn block_at(self, position: IVec3) -> Option<BlockId> {
        self.shape.block_at(position - self.ground)
    }

    pub(crate) fn minimum(self) -> IVec3 {
        self.ground + self.shape.minimum_offset()
    }

    pub(crate) fn maximum(self) -> IVec3 {
        self.ground + self.shape.maximum_offset()
    }

    pub(super) const fn shape(self) -> TreeShape {
        self.shape
    }
}
