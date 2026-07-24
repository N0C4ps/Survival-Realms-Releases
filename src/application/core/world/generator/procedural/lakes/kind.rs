use crate::application::core::blocks::BlockId;

use super::config;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LakeKind {
    Water,
    Lava,
}

impl LakeKind {
    pub(super) const ALL: [Self; 2] = [Self::Water, Self::Lava];

    pub(super) const fn block(self) -> BlockId {
        match self {
            Self::Water => BlockId::WATER,
            Self::Lava => BlockId::LAVA,
        }
    }

    pub(super) const fn spacing(self) -> i32 {
        match self {
            Self::Water => config::WATER_SPACING,
            Self::Lava => config::LAVA_SPACING,
        }
    }

    pub(super) const fn chance_percent(self) -> u64 {
        match self {
            Self::Water => config::WATER_CHANCE_PERCENT,
            Self::Lava => config::LAVA_CHANCE_PERCENT,
        }
    }

    pub(super) const fn radius_range(self) -> (i32, i32) {
        match self {
            Self::Water => (config::WATER_MIN_RADIUS, config::WATER_MAX_RADIUS),
            Self::Lava => (config::LAVA_MIN_RADIUS, config::LAVA_MAX_RADIUS),
        }
    }
}
