mod atomic_file;
mod paths;

use std::{fs, io::ErrorKind, path::PathBuf, time::SystemTime};

use super::{codec, format::LevelSnapshot};

#[derive(Clone)]
pub(super) struct LevelStorage {
    path: PathBuf,
}

impl LevelStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn load(&self) -> Result<Option<LevelSnapshot>, String> {
        self.recover_backup_if_needed()?;
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        codec::decode(&bytes).map(Some)
    }

    pub fn save(&self, level: &LevelSnapshot) -> Result<(), String> {
        let bytes = codec::encode(level)?;
        atomic_file::replace(&self.path, &bytes)
    }

    pub fn quarantine_corrupt_level(&self) -> Result<Option<PathBuf>, String> {
        if !self.path.exists() {
            return Ok(None);
        }
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs();
        let corrupt = self
            .path
            .with_extension(format!("level.corrupt-{timestamp}"));
        fs::rename(&self.path, &corrupt).map_err(|error| error.to_string())?;
        Ok(Some(corrupt))
    }

    fn recover_backup_if_needed(&self) -> Result<(), String> {
        let backup = paths::backup_path(&self.path);
        if !self.path.exists() && backup.exists() {
            fs::rename(&backup, &self.path).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}
