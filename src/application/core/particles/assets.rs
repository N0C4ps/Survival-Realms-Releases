use crate::application::core::blocks::BlockId;

pub(super) const TEXTURE_SIZE: u32 = 16;

pub(super) const fn texture_path(block: BlockId) -> Option<&'static str> {
    match block {
        BlockId::STONE => Some("particle/Stone_Particle.png"),
        BlockId::COBBLESTONE => Some("particle/Cobblestone_Particle.png"),
        BlockId::DIRT => Some("particle/Dirt_Particle.png"),
        BlockId::GRASS => Some("particle/Grass_Particle.png"),
        BlockId::PLANKS => Some("particle/Wood_Particle.png"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::core::paths::GamePaths;

    #[test]
    fn every_breakable_block_has_a_particle_texture() {
        for block in [
            BlockId::STONE,
            BlockId::COBBLESTONE,
            BlockId::DIRT,
            BlockId::GRASS,
            BlockId::PLANKS,
        ] {
            let path = texture_path(block).expect("solid block particle texture");
            let paths = GamePaths::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")))
                .expect("manifest directory");
            assert!(
                paths.asset(path).is_file(),
                "missing particle texture: {path}"
            );
        }
    }
}
