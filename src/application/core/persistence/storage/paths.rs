use std::path::{Path, PathBuf};

pub(super) fn temporary_path(level: &Path) -> PathBuf {
    level.with_extension("level.tmp")
}

pub(super) fn backup_path(level: &Path) -> PathBuf {
    level.with_extension("level.bak")
}
