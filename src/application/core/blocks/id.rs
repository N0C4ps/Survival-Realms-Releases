use std::{error::Error, fmt};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockId(u8);

impl BlockId {
    pub const AIR: Self = Self(0);
    pub const STONE: Self = Self(1);
    pub const COBBLESTONE: Self = Self(2);
    pub const DIRT: Self = Self(3);
    pub const GRASS: Self = Self(4);
    pub const PLANKS: Self = Self(5);
    /// Compatibility alias for code and saves created before wood became planks.
    pub const WOOD: Self = Self::PLANKS;
    pub const WATER: Self = Self(6);
    pub const LAVA: Self = Self(7);
    pub const CLAY: Self = Self(8);
    pub const SAND: Self = Self(9);
    pub const GRAVEL: Self = Self(10);
    pub const LEAVES: Self = Self(11);
    pub const LEAVE: Self = Self::LEAVES;
    pub const WOOD_LOG: Self = Self(12);

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn is_liquid(self) -> bool {
        self.0 == Self::WATER.0 || self.0 == Self::LAVA.0
    }
}

impl From<BlockId> for u8 {
    fn from(id: BlockId) -> Self {
        id.value()
    }
}

impl TryFrom<u8> for BlockId {
    type Error = UnknownBlockId;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0..=12 => Ok(Self(value)),
            _ => Err(UnknownBlockId(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnknownBlockId(u8);

impl fmt::Display for UnknownBlockId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown block id: {}", self.0)
    }
}

impl Error for UnknownBlockId {}
