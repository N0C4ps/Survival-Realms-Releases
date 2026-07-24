use super::{BlockDefinition, BlockId, BlockProperties, BlockTextures, TexturePath};

const BLOCK_COUNT: usize = 13;

pub struct BlockRegistry {
    definitions: [BlockDefinition; BLOCK_COUNT],
}

impl BlockRegistry {
    pub const fn builtin() -> Self {
        Self {
            definitions: [
                BlockDefinition::new(
                    BlockId::AIR,
                    "air",
                    BlockProperties::AIR,
                    BlockTextures::None,
                ),
                BlockDefinition::new(
                    BlockId::STONE,
                    "stone",
                    BlockProperties::SOLID,
                    BlockTextures::uniform(TexturePath::STONE),
                ),
                BlockDefinition::new(
                    BlockId::COBBLESTONE,
                    "cobblestone",
                    BlockProperties::SOLID,
                    BlockTextures::uniform(TexturePath::COBBLESTONE),
                ),
                BlockDefinition::new(
                    BlockId::DIRT,
                    "dirt",
                    BlockProperties::SOLID,
                    BlockTextures::uniform(TexturePath::DIRT),
                ),
                BlockDefinition::new(
                    BlockId::GRASS,
                    "grass",
                    BlockProperties::SOLID,
                    BlockTextures::faces(
                        TexturePath::GRASS_SIDE,
                        TexturePath::GRASS,
                        TexturePath::DIRT,
                    ),
                ),
                BlockDefinition::new(
                    BlockId::PLANKS,
                    "planks",
                    BlockProperties::SOLID,
                    BlockTextures::uniform(TexturePath::PLANKS),
                ),
                BlockDefinition::new(
                    BlockId::WATER,
                    "water",
                    BlockProperties::LIQUID,
                    BlockTextures::uniform(TexturePath::WATER),
                ),
                BlockDefinition::new(
                    BlockId::LAVA,
                    "lava",
                    BlockProperties::LIQUID,
                    BlockTextures::uniform(TexturePath::LAVA),
                ),
                BlockDefinition::new(
                    BlockId::CLAY,
                    "clay",
                    BlockProperties::SOLID,
                    BlockTextures::uniform(TexturePath::CLAY),
                ),
                BlockDefinition::new(
                    BlockId::SAND,
                    "sand",
                    BlockProperties::SOLID,
                    BlockTextures::uniform(TexturePath::SAND),
                ),
                BlockDefinition::new(
                    BlockId::GRAVEL,
                    "gravel",
                    BlockProperties::SOLID,
                    BlockTextures::uniform(TexturePath::GRAVEL),
                ),
                BlockDefinition::new(
                    BlockId::LEAVES,
                    "leaves",
                    BlockProperties::CUTOUT,
                    BlockTextures::uniform(TexturePath::LEAVES),
                ),
                BlockDefinition::new(
                    BlockId::WOOD_LOG,
                    "wood_log",
                    BlockProperties::SOLID,
                    BlockTextures::faces(
                        TexturePath::WOOD_LOG,
                        TexturePath::WOOD_LOG_TOP,
                        TexturePath::WOOD_LOG_TOP,
                    ),
                ),
            ],
        }
    }

    pub fn get(&self, id: BlockId) -> &BlockDefinition {
        &self.definitions[usize::from(id.value())]
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &BlockDefinition> {
        self.definitions.iter()
    }

    pub const fn len(&self) -> usize {
        self.definitions.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn validate(&self) {
        for (expected_id, block) in self.iter().enumerate() {
            debug_assert_eq!(usize::from(block.id().value()), expected_id);
            debug_assert!(!block.name().is_empty());

            let properties = block.properties();
            if properties.is_renderable() {
                for texture in block.textures().paths() {
                    let texture = texture.expect("renderable block faces must have textures");
                    debug_assert!(!texture.as_str().is_empty());
                }
            }
        }

        let air = self.get(BlockId::AIR);
        debug_assert!(air.properties().is_replaceable());
        debug_assert_eq!(air.textures(), BlockTextures::None);
    }
}

impl Default for BlockRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::application::core::blocks::TextureFace;

    #[test]
    fn builtin_blocks_have_stable_unique_ids() {
        let registry = BlockRegistry::builtin();
        let ids: HashSet<_> = registry.iter().map(|block| block.id()).collect();

        assert_eq!(registry.len(), BLOCK_COUNT);
        assert_eq!(ids.len(), BLOCK_COUNT);
        assert_eq!(registry.get(BlockId::AIR).name(), "air");
        assert_eq!(registry.get(BlockId::PLANKS).id().value(), 5);
        assert_eq!(registry.get(BlockId::WATER).id().value(), 6);
        assert_eq!(registry.get(BlockId::LAVA).id().value(), 7);
        assert_eq!(registry.get(BlockId::WOOD_LOG).id().value(), 12);
    }

    #[test]
    fn renderable_blocks_reference_existing_textures() {
        let registry = BlockRegistry::builtin();
        let paths = crate::application::core::paths::GamePaths::new(std::path::PathBuf::from(
            env!("CARGO_MANIFEST_DIR"),
        ))
        .expect("manifest directory");

        for block in registry
            .iter()
            .filter(|block| block.properties().is_renderable())
        {
            for texture in block.textures().paths().into_iter().flatten() {
                assert!(
                    paths.asset(texture.as_str()).is_file(),
                    "missing texture for {}: {}",
                    block.name(),
                    texture.as_str()
                );
            }
        }
    }

    #[test]
    fn air_is_non_solid_and_has_no_texture() {
        let air = *BlockRegistry::builtin().get(BlockId::AIR);

        assert!(!air.properties().is_solid());
        assert!(!air.properties().is_opaque());
        assert!(air.properties().is_replaceable());
        assert_eq!(air.textures(), BlockTextures::None);
    }

    #[test]
    fn fluids_are_non_solid_and_transparent() {
        let registry = BlockRegistry::builtin();

        for block in [BlockId::WATER, BlockId::LAVA] {
            let properties = registry.get(block).properties();
            assert!(!properties.is_solid());
            assert!(!properties.is_opaque());
            assert!(properties.is_renderable());
        }
    }

    #[test]
    fn grass_logs_and_leaves_have_their_special_face_rules() {
        let registry = BlockRegistry::builtin();
        let grass = registry.get(BlockId::GRASS).textures();
        let log = registry.get(BlockId::WOOD_LOG).textures();

        assert_eq!(grass.path(TextureFace::Side), Some(TexturePath::GRASS_SIDE));
        assert_eq!(grass.path(TextureFace::Top), Some(TexturePath::GRASS));
        assert_eq!(grass.path(TextureFace::Bottom), Some(TexturePath::DIRT));
        assert_eq!(log.path(TextureFace::Side), Some(TexturePath::WOOD_LOG));
        assert_eq!(log.path(TextureFace::Top), Some(TexturePath::WOOD_LOG_TOP));
        assert!(!registry.get(BlockId::LEAVES).properties().is_opaque());
        assert!(registry.get(BlockId::LEAVES).properties().is_solid());
    }

    #[test]
    fn leaves_texture_contains_visible_and_transparent_pixels() {
        let paths = crate::application::core::paths::GamePaths::new(std::path::PathBuf::from(
            env!("CARGO_MANIFEST_DIR"),
        ))
        .expect("manifest directory");
        let image = image::open(paths.asset(TexturePath::LEAVES.as_str()))
            .expect("leaves texture")
            .into_rgba8();

        assert!(image.pixels().any(|pixel| pixel.0[3] < 128));
        assert!(image.pixels().any(|pixel| pixel.0[3] >= 128));
    }
}
