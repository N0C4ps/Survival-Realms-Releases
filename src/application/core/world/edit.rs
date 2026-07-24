use glam::IVec3;

use crate::application::core::blocks::BlockId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BlockChange {
    pub position: IVec3,
    pub previous: BlockId,
    pub current: BlockId,
}
