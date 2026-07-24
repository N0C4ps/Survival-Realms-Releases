use std::{
    fs::OpenOptions as SyncOpenOptions,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use futures_util::StreamExt;
use semver::Version;
use serde::Serialize;
use sg_format::{PackageLimits, PackageReader};
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};
use url::Url;

use crate::{LauncherError, LauncherPaths, Result, TrustedKeyring};

use super::{
    DownloadProgress, RemoteRepository, RemoteVersion,
    index::{MAX_PACKAGE_BYTES, decode_hash, validate_same_origin},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DownloadOutcome {
    pub version: Version,
    pub package_path: PathBuf,
    pub downloaded_bytes: u64,
    pub already_present: bool,
}

impl RemoteRepository {
    pub async fn download_version<F>(
        &self,
        remote: &RemoteVersion,
        paths: &LauncherPaths,
        keyring: &TrustedKeyring,
        mut on_progress: F,
    ) -> Result<DownloadOutcome>
    where
        F: FnMut(DownloadProgress) -> bool,
    {
        let expected_hash = decode_hash(&remote.package_sha256)?;
        let destination = paths
            .versions()
            .join(format!("{}.sg", remote.manifest.version));
        if destination.exists() {
            if hash_file(&destination)? == expected_hash {
                return Ok(DownloadOutcome {
                    version: remote.manifest.version.clone(),
                    package_path: destination,
                    downloaded_bytes: 0,
                    already_present: true,
                });
            }
            return Err(LauncherError::VersionFileConflict(destination));
        }

        fs::create_dir_all(paths.versions()).await?;
        let temporary = temporary_path(&destination);
        let result = self
            .download_to(remote, &temporary, expected_hash, &mut on_progress)
            .await;
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary).await;
            return Err(error);
        }
        validate_downloaded_package(&temporary, remote, keyring)?;
        fs::rename(&temporary, &destination).await?;
        Ok(DownloadOutcome {
            version: remote.manifest.version.clone(),
            package_path: destination,
            downloaded_bytes: remote.package_size,
            already_present: false,
        })
    }

    async fn download_to<F>(
        &self,
        remote: &RemoteVersion,
        temporary: &Path,
        expected_hash: [u8; 32],
        on_progress: &mut F,
    ) -> Result<()>
    where
        F: FnMut(DownloadProgress) -> bool,
    {
        let package_url = Url::parse(&remote.package_url)
            .map_err(|error| LauncherError::InvalidRemoteIndex(error.to_string()))?;
        validate_same_origin(self.index_url(), &package_url)?;
        let response = self
            .client()
            .get(package_url)
            .send()
            .await?
            .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|length| length != remote.package_size)
        {
            return Err(LauncherError::DownloadTooLarge);
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temporary)
            .await?;
        let mut stream = response.bytes_stream();
        let mut hasher = Sha256::new();
        let mut downloaded = 0_u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded = downloaded
                .checked_add(chunk.len() as u64)
                .ok_or(LauncherError::DownloadTooLarge)?;
            if downloaded > remote.package_size || downloaded > MAX_PACKAGE_BYTES {
                return Err(LauncherError::DownloadTooLarge);
            }
            output.write_all(&chunk).await?;
            hasher.update(&chunk);
            if !on_progress(DownloadProgress {
                downloaded_bytes: downloaded,
                total_bytes: remote.package_size,
            }) {
                return Err(LauncherError::DownloadCancelled);
            }
        }
        output.flush().await?;
        output.sync_all().await?;
        if downloaded != remote.package_size || <[u8; 32]>::from(hasher.finalize()) != expected_hash
        {
            return Err(LauncherError::DownloadHashMismatch);
        }
        Ok(())
    }
}

fn validate_downloaded_package(
    path: &Path,
    remote: &RemoteVersion,
    keyring: &TrustedKeyring,
) -> Result<()> {
    let package = PackageReader::open(path, PackageLimits::default()).map_err(|error| {
        LauncherError::InvalidPackage {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })?;
    if package.manifest() != &remote.manifest {
        return Err(LauncherError::InvalidPackage {
            path: path.to_path_buf(),
            message: "package manifest differs from the signed remote index".to_owned(),
        });
    }
    let key = keyring
        .get(&package.signer_key_id())
        .ok_or_else(|| LauncherError::UnknownSigningKey(hex::encode(package.signer_key_id())))?;
    package
        .verify_signature(key)
        .and_then(|()| package.verify_payload())
        .map_err(|error| LauncherError::InvalidPackage {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}

fn hash_file(path: &Path) -> Result<[u8; 32]> {
    let mut file = SyncOpenOptions::new().read(true).open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn temporary_path(destination: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    destination.with_file_name(format!(
        ".{}.download-{}-{nonce}.tmp",
        destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        std::process::id()
    ))
}
