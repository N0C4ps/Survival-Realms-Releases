use std::path::{Path, PathBuf};

use super::{SaveStatus, inspect_save, prepare_migration_backup};

pub(crate) fn prepare_save_for_launch(level: &Path) -> Result<Option<PathBuf>, String> {
    let inspection = inspect_save(level)?;
    match inspection.status() {
        SaveStatus::Missing | SaveStatus::Ready | SaveStatus::Corrupt => Ok(None),
        SaveStatus::MigrationRequired => {
            let version = inspection
                .format_version()
                .ok_or_else(|| "migrating save has no format version".to_owned())?;
            prepare_migration_backup(level, version).map(Some)
        }
        SaveStatus::NewerThanGame => Err(format!(
            "save format {} is newer than this game's format {}; refusing to modify it",
            inspection.format_version().unwrap_or_default(),
            super::super::SAVE_FORMAT_VERSION
        )),
        SaveStatus::UnsupportedLegacy => Err(format!(
            "save format {} is older than the minimum supported format {}; refusing to modify it",
            inspection.format_version().unwrap_or_default(),
            super::super::MIN_SUPPORTED_SAVE_FORMAT
        )),
    }
}
