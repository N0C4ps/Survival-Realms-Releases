use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum LauncherError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid package {path}: {message}")]
    InvalidPackage { path: PathBuf, message: String },
    #[error("package is signed by an unknown key: {0}")]
    UnknownSigningKey(String),
    #[error("invalid Ed25519 public key: {0}")]
    InvalidPublicKey(String),
    #[error(
        "version {version} targets {platform}/{architecture}, but this launcher is running on {current_platform}/{current_architecture}"
    )]
    IncompatiblePlatform {
        version: semver::Version,
        platform: String,
        architecture: String,
        current_platform: &'static str,
        current_architecture: &'static str,
    },
    #[error("version {0} is no longer present in the local catalog")]
    MissingVersion(semver::Version),
    #[error("game metadata command failed: {0}")]
    MetadataCommand(String),
    #[error("save is incompatible with this game version: {0}")]
    IncompatibleSave(String),
    #[error("launcher JSON response is invalid: {0}")]
    InvalidResponse(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("remote repository URL is not allowed: {0}")]
    UnsafeRepositoryUrl(String),
    #[error("remote index is invalid: {0}")]
    InvalidRemoteIndex(String),
    #[error("remote index signature is invalid")]
    InvalidIndexSignature,
    #[error("download exceeds its declared or configured size limit")]
    DownloadTooLarge,
    #[error("downloaded package hash does not match the signed index")]
    DownloadHashMismatch,
    #[error("download was cancelled")]
    DownloadCancelled,
    #[error("version file already exists with different contents: {0}")]
    VersionFileConflict(PathBuf),
    #[error("required asset pack is not installed: {0}")]
    MissingAssetPack(String),
    #[error("asset pack cache conflicts with the signed manifest: {0}")]
    AssetPackConflict(PathBuf),
    #[error("invalid asset path: {0}")]
    InvalidAssetPath(String),
}

pub type Result<T> = std::result::Result<T, LauncherError>;
