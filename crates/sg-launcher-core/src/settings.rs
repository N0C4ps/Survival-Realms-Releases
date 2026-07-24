use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::Write,
    path::Path,
};

use semver::Version;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{LauncherError, Result};

const SETTINGS_SCHEMA_VERSION: u32 = 1;
const MAX_SETTINGS: usize = 256;
const MAX_SETTING_NAME_BYTES: usize = 64;

#[derive(Debug, Deserialize)]
pub(crate) struct SettingsIndex {
    schema_version: u32,
    game_version: Version,
    required_settings: BTreeMap<String, RequiredSetting>,
}

#[derive(Debug, Deserialize)]
struct RequiredSetting {
    #[serde(rename = "type")]
    kind: SettingKind,
    default: Value,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SettingKind {
    Integer,
    Number,
    Boolean,
    String,
}

impl SettingsIndex {
    pub(crate) fn validate(&self, expected_version: &Version) -> Result<()> {
        if self.schema_version != SETTINGS_SCHEMA_VERSION || &self.game_version != expected_version
        {
            return Err(invalid("settings index has incompatible metadata"));
        }
        if self.required_settings.is_empty() || self.required_settings.len() > MAX_SETTINGS {
            return Err(invalid("settings index has an invalid setting count"));
        }
        let mut seen = HashSet::new();
        for (name, setting) in &self.required_settings {
            if !valid_setting_name(name) || !seen.insert(name) {
                return Err(invalid("settings index contains an invalid setting name"));
            }
            if !setting.kind.accepts(&setting.default) {
                return Err(invalid(format!(
                    "default value for setting {name} does not match its declared type"
                )));
            }
        }
        Ok(())
    }

    pub(crate) fn merge_into(self, path: &Path) -> Result<bool> {
        let (mut document, existed) = match fs::read(path) {
            Ok(bytes) => {
                let value: Value = serde_json::from_slice(&bytes)?;
                let Value::Object(document) = value else {
                    return Err(invalid("settings.json must contain a JSON object"));
                };
                (document, true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Map::new(), false),
            Err(error) => return Err(error.into()),
        };

        let mut changed = !existed;
        for (name, setting) in self.required_settings {
            if let serde_json::map::Entry::Vacant(entry) = document.entry(name) {
                entry.insert(setting.default);
                changed = true;
            }
        }
        if !changed {
            return Ok(false);
        }

        let mut bytes = serde_json::to_vec_pretty(&document)?;
        bytes.push(b'\n');
        atomic_replace(path, &bytes)?;
        Ok(true)
    }
}

impl SettingKind {
    fn accepts(self, value: &Value) -> bool {
        match self {
            Self::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
            Self::Number => value.is_number(),
            Self::Boolean => value.is_boolean(),
            Self::String => value.is_string(),
        }
    }
}

fn valid_setting_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_SETTING_NAME_BYTES
        && name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit() && index > 0
                || byte == b'_' && index > 0
        })
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("settings path has no parent directory"))?;
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension("json.launcher-tmp");
    let backup = path.with_extension("json.launcher-bak");

    let mut file = File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if path.exists() {
        fs::rename(path, &backup)?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(error.into());
    }
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> LauncherError {
    LauncherError::InvalidRemoteIndex(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> SettingsIndex {
        serde_json::from_str(
            r#"{
                "schema_version": 1,
                "game_version": "0.0.2",
                "required_settings": {
                    "fov": {"type": "integer", "default": 100},
                    "mouse_sensitivity": {"type": "integer", "default": 60},
                    "render_distance": {"type": "integer", "default": 8},
                    "brightness": {"type": "integer", "default": 75}
                }
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn validates_the_document_published_for_version_0_0_2() {
        assert!(index().validate(&Version::new(0, 0, 2)).is_ok());
        assert!(index().validate(&Version::new(0, 0, 3)).is_err());
    }

    #[test]
    fn merge_adds_only_missing_values_and_preserves_future_fields() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("settings.json");
        fs::write(&path, r#"{"mouse_sensitivity":70,"future_setting":true}"#).unwrap();

        assert!(index().merge_into(&path).unwrap());
        let document: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(document["mouse_sensitivity"], 70);
        assert_eq!(document["future_setting"], true);
        assert_eq!(document["fov"], 100);
        assert_eq!(document["render_distance"], 8);
        assert_eq!(document["brightness"], 75);
    }

    #[test]
    fn repeated_merge_does_not_rewrite_the_file() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("settings.json");
        assert!(index().merge_into(&path).unwrap());
        let original = fs::read(&path).unwrap();
        assert!(!index().merge_into(&path).unwrap());
        assert_eq!(fs::read(path).unwrap(), original);
    }
}
