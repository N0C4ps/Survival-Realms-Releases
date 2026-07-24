use std::time::Duration;

use crate::application::core::blocks::BlockId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FluidKind {
    Water,
    Lava,
}

impl FluidKind {
    pub(super) fn from_block(block: BlockId) -> Option<Self> {
        if block == BlockId::WATER {
            Some(Self::Water)
        } else if block == BlockId::LAVA {
            Some(Self::Lava)
        } else {
            None
        }
    }

    pub(super) const fn block(self) -> BlockId {
        match self {
            Self::Water => BlockId::WATER,
            Self::Lava => BlockId::LAVA,
        }
    }

    pub(super) const fn maximum_level(self) -> u8 {
        match self {
            Self::Water => 8,
            Self::Lava => 5,
        }
    }

    pub(super) const fn step_interval(self) -> Duration {
        match self {
            Self::Water => Duration::from_millis(500),
            Self::Lava => Duration::from_secs(1),
        }
    }
}
