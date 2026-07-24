use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use super::paths;

pub(super) fn replace(level: &Path, bytes: &[u8]) -> Result<(), String> {
    let directory = level.parent().ok_or("level path has no parent directory")?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let temporary = paths::temporary_path(level);
    let backup = paths::backup_path(level);

    let mut file = File::create(&temporary).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);

    if backup.exists() {
        fs::remove_file(&backup).map_err(|error| error.to_string())?;
    }
    if level.exists() {
        fs::rename(level, &backup).map_err(|error| error.to_string())?;
    }
    if let Err(error) = fs::rename(&temporary, level) {
        if backup.exists() {
            let _ = fs::rename(&backup, level);
        }
        return Err(error.to_string());
    }
    if backup.exists() {
        fs::remove_file(backup).map_err(|error| error.to_string())?;
    }
    Ok(())
}
