use std::path::Path;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{PackageError, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    Development,
    Snapshot,
    Release,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub build_identity_schema: u16,
    pub game_id: String,
    pub version: Version,
    pub display_name: String,
    pub channel: ReleaseChannel,
    pub platform: String,
    pub architecture: String,
    pub executable: String,
    pub asset_pack: String,
    pub minimum_save_format: u32,
    pub maximum_save_format: u32,
    pub generator_version: u8,
    pub protocol_version: u32,
    pub minimum_launcher_version: Version,
}

impl PackageManifest {
    pub(crate) fn validate(&self) -> Result<()> {
        validate_display_name(&self.display_name)?;
        for (name, value) in [
            ("game_id", self.game_id.as_str()),
            ("platform", self.platform.as_str()),
            ("architecture", self.architecture.as_str()),
            ("asset_pack", self.asset_pack.as_str()),
        ] {
            if !valid_identifier(value) {
                return Err(PackageError::InvalidManifest(format!(
                    "{name} must use only ASCII letters, numbers, '.', '_' or '-'"
                )));
            }
        }
        let executable = Path::new(&self.executable);
        if self.executable.is_empty()
            || self.executable.len() > 128
            || self.executable.contains(['/', '\\', ':'])
            || executable.is_absolute()
            || executable.components().count() != 1
        {
            return Err(PackageError::InvalidManifest(
                "executable must be a single relative file name".to_owned(),
            ));
        }
        if self.minimum_save_format > self.maximum_save_format {
            return Err(PackageError::InvalidManifest(
                "minimum_save_format exceeds maximum_save_format".to_owned(),
            ));
        }
        Ok(())
    }
}

fn validate_display_name(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(PackageError::InvalidManifest(
            "display_name must contain 1-128 printable bytes".to_owned(),
        ));
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
