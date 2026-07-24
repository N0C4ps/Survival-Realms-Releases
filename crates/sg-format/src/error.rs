use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid .sg signature")]
    InvalidMagic,
    #[error("unsupported .sg format version {0}")]
    UnsupportedFormat(u16),
    #[error("unsupported .sg flags 0x{0:04x}")]
    UnsupportedFlags(u16),
    #[error(".sg package is truncated or has trailing data")]
    InvalidFileLength,
    #[error("{field} exceeds its safety limit: {actual} > {maximum}")]
    LimitExceeded {
        field: &'static str,
        actual: u64,
        maximum: u64,
    },
    #[error("manifest hash does not match its header")]
    ManifestHashMismatch,
    #[error("payload hash does not match its header")]
    PayloadHashMismatch,
    #[error("executable hash does not match its header")]
    ExecutableHashMismatch,
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid signed version index: {0}")]
    InvalidIndex(String),
    #[error("package was not signed by the expected key")]
    UnexpectedSigningKey,
    #[error("package signature is invalid")]
    InvalidSignature,
    #[error("destination already contains a different executable: {0}")]
    DestinationConflict(PathBuf),
    #[error("package destination already exists: {0}")]
    PackageAlreadyExists(PathBuf),
}

pub type Result<T> = std::result::Result<T, PackageError>;

impl From<serde_json::Error> for PackageError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidManifest(error.to_string())
    }
}
