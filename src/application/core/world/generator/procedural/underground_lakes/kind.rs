use crate::application::core::blocks::BlockId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UndergroundLakeKind {
    Water,
    Lava,
}

impl UndergroundLakeKind {
    pub(crate) const fn block(self) -> BlockId {
        match self {
            Self::Water => BlockId::WATER,
            Self::Lava => BlockId::LAVA,
        }
    }
}
