use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde_json::{Map, Value};

use super::GameSettings;

pub(crate) struct SettingsStore {
    path: PathBuf,
    document: Map<String, Value>,
    pending: Option<(GameSettings, Instant)>,
}

impl SettingsStore {
    const SAVE_DELAY: Duration = Duration::from_millis(400);

    pub(crate) fn load(path: PathBuf) -> (Self, GameSettings) {
        let document = match read_document(&path) {
            Ok(document) => document,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Map::new(),
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "settings could not be read; defaults will be used"
                );
                Map::new()
            }
        };
        let mut settings = settings_from_document(&document);
        settings.clamp();
        let mut store = Self {
            path,
            document,
            pending: None,
        };

        if let Err(error) = store.save(settings) {
            tracing::warn!(%error, "settings could not be initialized");
        } else {
            tracing::info!(
                path = %store.path.display(),
                "settings loaded"
            );
        }
        (store, settings)
    }

    pub(crate) fn save(&mut self, settings: GameSettings) -> Result<(), String> {
        self.pending = None;
        let known = serde_json::to_value(settings)
            .map_err(|error| format!("failed to encode settings: {error}"))?;
        let Value::Object(known) = known else {
            return Err("settings did not encode as a JSON object".to_owned());
        };
        self.document.extend(known);
        let bytes = serde_json::to_vec_pretty(&self.document)
            .map_err(|error| format!("failed to encode settings: {error}"))?;
        atomic_replace(&self.path, &bytes)?;
        tracing::debug!(path = %self.path.display(), "settings saved");
        Ok(())
    }

    pub(crate) fn schedule_save(&mut self, settings: GameSettings) {
        self.pending = Some((settings, Instant::now()));
    }

    pub(crate) fn flush_if_due(&mut self) -> Result<(), String> {
        let Some((settings, changed_at)) = self.pending else {
            return Ok(());
        };
        if changed_at.elapsed() < Self::SAVE_DELAY {
            return Ok(());
        }
        self.save(settings)
    }

    pub(crate) fn flush(&mut self) -> Result<(), String> {
        let Some((settings, _)) = self.pending else {
            return Ok(());
        };
        self.save(settings)
    }
}

fn read_document(path: &Path) -> io::Result<Map<String, Value>> {
    let bytes = fs::read(path)?;
    serde_json::from_slice::<Map<String, Value>>(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn settings_from_document(document: &Map<String, Value>) -> GameSettings {
    let defaults = GameSettings::default();
    GameSettings {
        fov: unsigned(document, "fov")
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or(defaults.fov),
        mouse_sensitivity: unsigned(document, "mouse_sensitivity")
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(defaults.mouse_sensitivity),
        render_distance: unsigned(document, "render_distance")
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(defaults.render_distance),
        brightness: unsigned(document, "brightness")
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(defaults.brightness),
    }
}

fn unsigned(document: &Map<String, Value>, key: &str) -> Option<u64> {
    document.get(key).and_then(Value::as_u64)
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or_else(|| "settings path has no parent directory".to_owned())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("failed to create settings directory: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");

    let mut file = File::create(&temporary)
        .map_err(|error| format!("failed to create temporary settings: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("failed to write settings: {error}"))?;
    file.write_all(b"\n")
        .map_err(|error| format!("failed to finish settings: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("failed to flush settings: {error}"))?;
    drop(file);

    if backup.exists() {
        fs::remove_file(&backup)
            .map_err(|error| format!("failed to remove old settings backup: {error}"))?;
    }
    if path.exists() {
        fs::rename(path, &backup)
            .map_err(|error| format!("failed to back up settings: {error}"))?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(format!("failed to replace settings: {error}"));
    }
    if backup.exists() {
        fs::remove_file(backup)
            .map_err(|error| format!("failed to remove settings backup: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("survival-realms-settings-{name}-{unique}"))
            .join("settings.json")
    }

    #[test]
    fn missing_file_is_created_with_defaults() {
        let path = test_path("defaults");
        let (_, settings) = SettingsStore::load(path.clone());

        assert_eq!(settings, GameSettings::default());
        assert!(path.is_file());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_clamps_values_and_preserves_unknown_keys_when_saving() {
        let path = test_path("merge");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"fov":999,"mouse_sensitivity":70,"future_option":true}"#,
        )
        .unwrap();

        let (mut store, mut settings) = SettingsStore::load(path.clone());
        assert_eq!(settings.fov, 200);
        assert_eq!(settings.mouse_sensitivity, 70);
        settings.brightness = 50;
        store.save(settings).unwrap();

        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["future_option"], true);
        assert_eq!(saved["brightness"], 50);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
