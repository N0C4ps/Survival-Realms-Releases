use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use futures_util::StreamExt;
use serde::Serialize;
use sg_format::{RemoteAssetFile, RemoteAssetPack};
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};
use url::Url;

use crate::{LauncherError, LauncherPaths, Result};

use super::{
    DownloadProgress, RemoteRepository,
    index::{MAX_ASSET_FILE_BYTES, decode_hash, validate_relative_path, validate_same_origin},
};

pub(crate) const PACK_MARKER: &str = ".sg-pack.json";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AssetPackDownloadOutcome {
    pub id: String,
    pub cache_path: PathBuf,
    pub downloaded_bytes: u64,
    pub already_present: bool,
}

impl RemoteRepository {
    pub async fn download_asset_pack<F>(
        &self,
        pack: &RemoteAssetPack,
        paths: &LauncherPaths,
        mut on_progress: F,
    ) -> Result<AssetPackDownloadOutcome>
    where
        F: FnMut(DownloadProgress) -> bool,
    {
        let destination = paths.asset_packs().join(&pack.id);
        if destination.exists() {
            if cached_manifest(&destination).as_ref() == Some(pack) {
                return Ok(AssetPackDownloadOutcome {
                    id: pack.id.clone(),
                    cache_path: destination,
                    downloaded_bytes: 0,
                    already_present: true,
                });
            }
            return Err(LauncherError::AssetPackConflict(destination));
        }

        fs::create_dir_all(paths.asset_packs()).await?;
        let staging = temporary_directory(&destination);
        fs::create_dir(&staging).await?;
        let total_bytes = pack.total_size().ok_or(LauncherError::DownloadTooLarge)?;
        let mut completed_bytes = 0_u64;
        let result = async {
            for file in &pack.files {
                let target = staging.join(&file.path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).await?;
                }
                self.download_asset_file(
                    file,
                    &target,
                    completed_bytes,
                    total_bytes,
                    &mut on_progress,
                )
                .await?;
                completed_bytes = completed_bytes
                    .checked_add(file.file_size)
                    .ok_or(LauncherError::DownloadTooLarge)?;
            }
            let marker = serde_json::to_vec_pretty(pack)?;
            let marker_path = staging.join(PACK_MARKER);
            let mut marker_file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(marker_path)
                .await?;
            marker_file.write_all(&marker).await?;
            marker_file.sync_all().await?;
            drop(marker_file);
            fs::rename(&staging, &destination).await?;
            Ok::<(), LauncherError>(())
        }
        .await;
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&staging).await;
            return Err(error);
        }
        Ok(AssetPackDownloadOutcome {
            id: pack.id.clone(),
            cache_path: destination,
            downloaded_bytes: completed_bytes,
            already_present: false,
        })
    }

    async fn download_asset_file<F>(
        &self,
        file: &RemoteAssetFile,
        target: &Path,
        completed_bytes: u64,
        total_bytes: u64,
        on_progress: &mut F,
    ) -> Result<()>
    where
        F: FnMut(DownloadProgress) -> bool,
    {
        validate_relative_path(&file.path)?;
        let expected_hash = decode_hash(&file.file_sha256)?;
        let file_url = Url::parse(&file.file_url)
            .map_err(|error| LauncherError::InvalidRemoteIndex(error.to_string()))?;
        validate_same_origin(self.index_url(), &file_url)?;
        let response = self
            .client()
            .get(file_url)
            .send()
            .await?
            .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|length| length != file.file_size)
        {
            return Err(LauncherError::DownloadHashMismatch);
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .await?;
        let mut stream = response.bytes_stream();
        let mut hasher = Sha256::new();
        let mut downloaded = 0_u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded = downloaded
                .checked_add(chunk.len() as u64)
                .ok_or(LauncherError::DownloadTooLarge)?;
            if downloaded > file.file_size || downloaded > MAX_ASSET_FILE_BYTES {
                return Err(LauncherError::DownloadTooLarge);
            }
            output.write_all(&chunk).await?;
            hasher.update(&chunk);
            if !on_progress(DownloadProgress {
                downloaded_bytes: completed_bytes + downloaded,
                total_bytes,
            }) {
                return Err(LauncherError::DownloadCancelled);
            }
        }
        output.flush().await?;
        output.sync_all().await?;
        if downloaded != file.file_size || <[u8; 32]>::from(hasher.finalize()) != expected_hash {
            return Err(LauncherError::DownloadHashMismatch);
        }
        Ok(())
    }
}

pub(crate) fn cached_manifest(directory: &Path) -> Option<RemoteAssetPack> {
    let bytes = std::fs::read(directory.join(PACK_MARKER)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn temporary_directory(destination: &Path) -> PathBuf {
    destination.with_file_name(format!(
        ".{}.download-{}-{}.tmp",
        destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}
