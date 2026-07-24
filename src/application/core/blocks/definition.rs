use super::{BlockId, BlockProperties, BlockTextures};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockDefinition {
    id: BlockId,
    name: &'static str,
    properties: BlockProperties,
    textures: BlockTextures,
}

impl BlockDefinition {
    pub const fn new(
        id: BlockId,
        name: &'static str,
        properties: BlockProperties,
        textures: BlockTextures,
    ) -> Self {
        Self {
            id,
            name,
            properties,
            textures,
        }
    }

    pub const fn id(self) -> BlockId {
        self.id
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn properties(self) -> BlockProperties {
        self.properties
    }

    pub const fn textures(self) -> BlockTextures {
        self.textures
    }
}
