use std::{
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

pub(crate) fn prepare_migration_backup(
    level: &Path,
    source_version: u32,
) -> Result<PathBuf, String> {
    let file_name = level
        .file_name()
        .ok_or_else(|| "level path has no file name".to_owned())?
        .to_string_lossy();
    let backup = level.with_file_name(format!("{file_name}.pre-migration-v{source_version}.bak"));

    let source = File::open(level).map_err(|error| error.to_string())?;
    let mut destination = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)
    {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => return Ok(backup),
        Err(error) => return Err(error.to_string()),
    };
    let mut source = io::BufReader::new(source);
    if let Err(error) = io::copy(&mut source, &mut destination) {
        drop(destination);
        let _ = fs::remove_file(&backup);
        return Err(error.to_string());
    }
    destination.sync_all().map_err(|error| error.to_string())?;
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    #[test]
    fn migration_backup_is_idempotent_and_preserves_original_bytes() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let level = std::env::temp_dir().join(format!("sg-backup-{unique}.level"));
        fs::write(&level, b"original level").unwrap();

        let first = prepare_migration_backup(&level, 2).unwrap();
        let second = prepare_migration_backup(&level, 2).unwrap();

        assert_eq!(first, second);
        assert_eq!(fs::read(&first).unwrap(), b"original level");
        fs::remove_file(level).unwrap();
        fs::remove_file(first).unwrap();
    }
}
