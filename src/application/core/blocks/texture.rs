#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TexturePath(&'static str);

impl TexturePath {
    pub const STONE: Self = Self("texture/Stone_Block.png");
    pub const COBBLESTONE: Self = Self("texture/Cobblestone_Block.png");
    pub const DIRT: Self = Self("texture/Dirt_Block.png");
    pub const GRASS: Self = Self("texture/Grass_Block.png");
    pub const GRASS_SIDE: Self = Self("texture/Grass_Block_Side.png");
    pub const PLANKS: Self = Self("texture/Plains_Block.png");
    pub const WATER: Self = Self("texture/Water.png");
    pub const LAVA: Self = Self("texture/Lava.png");
    pub const CLAY: Self = Self("texture/Clay_Block.png");
    pub const SAND: Self = Self("texture/Sand_Block.png");
    pub const GRAVEL: Self = Self("texture/Gravel_Block.png");
    pub const LEAVES: Self = Self("texture/Leave_Block.png");
    pub const WOOD_LOG: Self = Self("texture/Wood_Log.png");
    pub const WOOD_LOG_TOP: Self = Self("texture/Wood_Log_Top.png");

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockTextures {
    None,
    Uniform(TexturePath),
    Faces {
        side: TexturePath,
        top: TexturePath,
        bottom: TexturePath,
    },
}

impl BlockTextures {
    pub const fn uniform(path: TexturePath) -> Self {
        Self::Uniform(path)
    }

    pub const fn faces(side: TexturePath, top: TexturePath, bottom: TexturePath) -> Self {
        Self::Faces { side, top, bottom }
    }

    pub const fn path(self, face: TextureFace) -> Option<TexturePath> {
        match self {
            Self::None => None,
            Self::Uniform(path) => Some(path),
            Self::Faces { side, top, bottom } => Some(match face {
                TextureFace::Side => side,
                TextureFace::Top => top,
                TextureFace::Bottom => bottom,
            }),
        }
    }

    pub const fn paths(self) -> [Option<TexturePath>; TEXTURES_PER_BLOCK as usize] {
        [
            self.path(TextureFace::Side),
            self.path(TextureFace::Top),
            self.path(TextureFace::Bottom),
        ]
    }
}

pub const TEXTURES_PER_BLOCK: u32 = 3;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureFace {
    Side = 0,
    Top = 1,
    Bottom = 2,
}

impl TextureFace {
    pub const fn layer(self, block: super::BlockId) -> u32 {
        block.value() as u32 * TEXTURES_PER_BLOCK + self as u32
    }
}
