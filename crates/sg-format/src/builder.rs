use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signer, SigningKey};

use crate::{
    PackageError, PackageLimits, PackageManifest, Result,
    crypto::{key_id, sha256},
    header::{FLAG_SIGNED, Header, SIGNATURE_SIZE},
};

pub struct PackageBuilder<'a> {
    manifest: &'a PackageManifest,
    executable: &'a [u8],
    compression_level: i32,
}

impl<'a> PackageBuilder<'a> {
    pub fn new(manifest: &'a PackageManifest, executable: &'a [u8]) -> Self {
        Self {
            manifest,
            executable,
            compression_level: 19,
        }
    }

    pub fn compression_level(mut self, level: i32) -> Self {
        self.compression_level = level;
        self
    }

    pub fn write_signed(self, path: impl AsRef<Path>, signing_key: &SigningKey) -> Result<()> {
        let path = path.as_ref();
        if path.exists() {
            return Err(PackageError::PackageAlreadyExists(path.to_path_buf()));
        }
        self.manifest.validate()?;
        let manifest = serde_json::to_vec(self.manifest)?;
        let payload = zstd::stream::encode_all(self.executable, self.compression_level)?;
        let limits = PackageLimits::default();
        validate_build_lengths(manifest.len(), payload.len(), self.executable.len(), limits)?;

        let mut header = Header {
            flags: FLAG_SIGNED,
            manifest_len: manifest.len() as u32,
            payload_len: payload.len() as u64,
            executable_len: self.executable.len() as u64,
            manifest_hash: sha256(&manifest),
            payload_hash: sha256(&payload),
            executable_hash: sha256(self.executable),
            key_id: key_id(&signing_key.verifying_key()),
            signature: [0; SIGNATURE_SIZE],
        };
        header.signature = signing_key
            .sign(&header.signed_message(&manifest))
            .to_bytes();

        let temporary = temporary_path(path);
        let result = (|| {
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            header.write(&mut output)?;
            output.write_all(&manifest)?;
            output.write_all(&payload)?;
            output.sync_all()?;
            fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn temporary_path(destination: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    destination.with_file_name(format!(".{name}.sg-build-{}-{nonce}", std::process::id()))
}

fn validate_build_lengths(
    manifest: usize,
    payload: usize,
    executable: usize,
    limits: PackageLimits,
) -> Result<()> {
    for (field, actual, maximum) in [
        (
            "manifest",
            manifest as u64,
            u64::from(limits.max_manifest_bytes),
        ),
        (
            "compressed payload",
            payload as u64,
            limits.max_compressed_bytes,
        ),
        ("executable", executable as u64, limits.max_executable_bytes),
    ] {
        if actual > maximum {
            return Err(PackageError::LimitExceeded {
                field,
                actual,
                maximum,
            });
        }
    }
    Ok(())
}
