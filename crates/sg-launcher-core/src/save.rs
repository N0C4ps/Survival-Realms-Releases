use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{LauncherError, LauncherPaths, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveStatus {
    Missing,
    Ready,
    MigrationRequired,
    NewerThanGame,
    UnsupportedLegacy,
    Corrupt,
}

impl std::fmt::Display for SaveStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "missing",
            Self::Ready => "ready",
            Self::MigrationRequired => "migration_required",
            Self::NewerThanGame => "newer_than_game",
            Self::UnsupportedLegacy => "unsupported_legacy",
            Self::Corrupt => "corrupt",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SaveInspection {
    pub schema_version: u16,
    pub path: String,
    pub status: SaveStatus,
    pub format_version: Option<u32>,
    pub current_format: u32,
    pub minimum_supported_format: u32,
    pub message: Option<String>,
}

pub(crate) fn inspect(executable: &Path, paths: &LauncherPaths) -> Result<SaveInspection> {
    fs::create_dir_all(paths.launcher_data())?;
    let output_path = metadata_output_path(paths);
    let output = metadata_command(executable, paths.root(), &output_path)?;
    let inspection: SaveInspection = serde_json::from_slice(&output)?;
    match inspection.status {
        SaveStatus::NewerThanGame | SaveStatus::UnsupportedLegacy => {
            Err(LauncherError::IncompatibleSave(format!(
                "status {:?}, save format {:?}, supported {}..={}",
                inspection.status,
                inspection.format_version,
                inspection.minimum_supported_format,
                inspection.current_format
            )))
        }
        _ => Ok(inspection),
    }
}

fn metadata_command(executable: &Path, game_dir: &Path, output_path: &Path) -> Result<Vec<u8>> {
    let mut command = Command::new(executable);
    command
        .arg("--inspect-save")
        .arg("--game-dir")
        .arg(game_dir)
        .arg("--metadata-output")
        .arg(output_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    configure_no_window(&mut command);
    let output = command
        .output()
        .map_err(|error| LauncherError::MetadataCommand(error.to_string()))?;
    if output.status.success()
        && let Ok(metadata) = fs::read(output_path)
        && !metadata.is_empty()
    {
        let _ = fs::remove_file(output_path);
        return Ok(metadata);
    }
    let file_error = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let _ = fs::remove_file(output_path);

    let mut legacy = Command::new(executable);
    legacy
        .arg("--inspect-save")
        .arg("--game-dir")
        .arg(game_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_no_window(&mut legacy);
    let legacy_output = legacy
        .output()
        .map_err(|error| LauncherError::MetadataCommand(error.to_string()))?;
    if legacy_output.status.success() && !legacy_output.stdout.is_empty() {
        return Ok(legacy_output.stdout);
    }
    let legacy_error = String::from_utf8_lossy(&legacy_output.stderr)
        .trim()
        .to_owned();
    Err(LauncherError::MetadataCommand(format!(
        "metadata file failed: {}; legacy output failed: {}",
        diagnostic(&file_error),
        diagnostic(&legacy_error)
    )))
}

fn configure_no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

fn diagnostic(message: &str) -> &str {
    if message.is_empty() {
        "no diagnostic output"
    } else {
        message
    }
}

fn metadata_output_path(paths: &LauncherPaths) -> PathBuf {
    paths.launcher_data().join(format!(
        ".save-metadata-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}
