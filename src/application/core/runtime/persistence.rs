use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use super::state::Runtime;
use crate::application::core::{
    persistence::LevelSnapshot,
    world::{TerrainDimensions, World, WorldGenerator},
};
use glam::Vec3;

impl Runtime {
    pub(super) fn load_or_create_world(&mut self) -> (World, Option<Vec3>) {
        match self.persistence.load() {
            Ok(Some(level)) => match restore_level(&level) {
                Ok((world, generator, restored, player_position)) => {
                    self.terrain_generator = generator;
                    tracing::info!(
                        seed = generator.seed(),
                        restored_blocks = restored,
                        path = %self.persistence.level_path().display(),
                        "level loaded"
                    );
                    return (world, player_position);
                }
                Err(error) => self.handle_corrupt_level(&error),
            },
            Ok(None) => tracing::info!(
                path = %self.persistence.level_path().display(),
                "no level found; creating a new world"
            ),
            Err(error) => self.handle_corrupt_level(&error),
        }

        let seed = generate_seed();
        self.terrain_generator = WorldGenerator::procedural(TerrainDimensions::default(), seed);
        let snapshot = LevelSnapshot::capture(&World::default(), self.terrain_generator, None);
        match self.persistence.save_async(snapshot) {
            Ok(()) => match self.persistence.flush() {
                Ok(()) => tracing::info!(seed, "initial level file created"),
                Err(error) => tracing::error!(%error, "failed to finish initial level save"),
            },
            Err(error) => tracing::error!(%error, "failed to create initial level file"),
        }
        tracing::info!(seed, "new level created");
        (World::default(), None)
    }

    pub(super) fn request_level_save(&mut self) {
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let player_position = self.player.as_ref().map(|player| player.position());
        let snapshot = LevelSnapshot::capture(world, self.terrain_generator, player_position);
        match self.persistence.save_async(snapshot) {
            Ok(()) => {
                self.level_dirty = false;
                tracing::info!("level save queued");
            }
            Err(error) => tracing::error!(%error, "failed to queue level save"),
        }
    }

    pub(super) fn save_level_before_exit(&mut self) {
        self.request_level_save();
        if let Err(error) = self.persistence.flush() {
            tracing::error!(%error, "failed to finish level save before exit");
        }
        if let Err(error) = self.settings_store.flush() {
            tracing::error!(%error, "failed to finish settings save before exit");
        }
    }

    fn handle_corrupt_level(&self, error: &str) {
        tracing::error!(%error, "level could not be loaded");
        match self.persistence.quarantine_corrupt_level() {
            Ok(Some(path)) => tracing::warn!(path = %path.display(), "corrupt level preserved"),
            Ok(None) => {}
            Err(quarantine_error) => {
                tracing::error!(%quarantine_error, "failed to preserve corrupt level")
            }
        }
    }
}

fn restore_level(
    level: &LevelSnapshot,
) -> Result<(World, WorldGenerator, usize, Option<Vec3>), String> {
    let generator = level.generator()?;
    let mut world = World::default();
    let restored = level.restore_into(&mut world)?;
    let player_position = level.player_position()?;
    Ok((world, generator, restored, player_position))
}

fn generate_seed() -> u64 {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
    (nanos as u64)
        ^ ((nanos >> 64) as u64).rotate_left(17)
        ^ sequence.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_new_world_seed_is_unique_within_the_process() {
        let first = generate_seed();
        let second = generate_seed();
        assert_ne!(first, second);
    }
}
