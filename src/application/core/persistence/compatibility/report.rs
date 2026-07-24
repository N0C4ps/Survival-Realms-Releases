use std::path::Path;

use serde::Serialize;

use super::super::{MIN_SUPPORTED_SAVE_FORMAT, SAVE_FORMAT_VERSION};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SaveStatus {
    Missing,
    Ready,
    MigrationRequired,
    NewerThanGame,
    UnsupportedLegacy,
    Corrupt,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct SaveInspection {
    schema_version: u16,
    path: String,
    status: SaveStatus,
    format_version: Option<u32>,
    current_format: u32,
    minimum_supported_format: u32,
    message: Option<String>,
}

impl SaveInspection {
    pub(super) fn missing(path: &Path) -> Self {
        Self::new(path, SaveStatus::Missing, None, None)
    }

    pub(super) fn from_version(path: &Path, version: u32) -> Self {
        let status = if version == SAVE_FORMAT_VERSION {
            SaveStatus::Ready
        } else if (MIN_SUPPORTED_SAVE_FORMAT..SAVE_FORMAT_VERSION).contains(&version) {
            SaveStatus::MigrationRequired
        } else if version > SAVE_FORMAT_VERSION {
            SaveStatus::NewerThanGame
        } else {
            SaveStatus::UnsupportedLegacy
        };
        Self::new(path, status, Some(version), None)
    }

    pub(super) fn corrupt(path: &Path, message: impl Into<String>) -> Self {
        Self::new(path, SaveStatus::Corrupt, None, Some(message.into()))
    }

    fn new(
        path: &Path,
        status: SaveStatus,
        format_version: Option<u32>,
        message: Option<String>,
    ) -> Self {
        Self {
            schema_version: 1,
            path: path.to_string_lossy().into_owned(),
            status,
            format_version,
            current_format: SAVE_FORMAT_VERSION,
            minimum_supported_format: MIN_SUPPORTED_SAVE_FORMAT,
            message,
        }
    }

    pub(crate) fn status(&self) -> SaveStatus {
        self.status
    }

    pub(crate) fn format_version(&self) -> Option<u32> {
        self.format_version
    }

    pub(crate) fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
