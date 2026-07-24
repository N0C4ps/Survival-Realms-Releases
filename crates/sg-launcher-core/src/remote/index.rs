use std::{collections::HashSet, path::Component};

use sg_format::{
    INDEX_SCHEMA_VERSION, RemoteAssetFile, RemoteAssetPack, RemoteVersion, SignedVersionIndex,
    VersionIndex,
};
use url::Url;

use crate::{LauncherError, Result, TrustedKeyring};

pub(super) const MAX_INDEX_BYTES: usize = 2 * 1024 * 1024;
pub(super) const MAX_PACKAGE_BYTES: u64 = 300 * 1024 * 1024;
pub(super) const MAX_ASSET_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub(super) const MAX_ASSET_PACK_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ASSET_FILES: usize = 16_384;

pub(super) fn verify_index(
    signed: SignedVersionIndex,
    keyring: &TrustedKeyring,
    index_url: &Url,
) -> Result<VersionIndex> {
    let key_id = signed
        .signer_key_id()
        .map_err(|error| LauncherError::InvalidRemoteIndex(error.to_string()))?;
    let key = keyring
        .get(&key_id)
        .ok_or_else(|| LauncherError::UnknownSigningKey(hex::encode(key_id)))?;
    signed
        .verify(key)
        .map_err(|_| LauncherError::InvalidIndexSignature)?;
    validate_index(signed.index, index_url)
}

fn validate_index(mut index: VersionIndex, index_url: &Url) -> Result<VersionIndex> {
    if index.schema_version != INDEX_SCHEMA_VERSION {
        return Err(LauncherError::InvalidRemoteIndex(format!(
            "unsupported schema {}",
            index.schema_version
        )));
    }
    let mut versions = HashSet::new();
    for entry in &index.versions {
        validate_version(entry, index_url)?;
        if !versions.insert(entry.manifest.version.clone()) {
            return Err(LauncherError::InvalidRemoteIndex(format!(
                "duplicate version {}",
                entry.manifest.version
            )));
        }
    }

    let mut pack_ids = HashSet::new();
    for pack in &index.asset_packs {
        validate_asset_pack(pack, index_url)?;
        if !pack_ids.insert(pack.id.as_str()) {
            return Err(LauncherError::InvalidRemoteIndex(format!(
                "duplicate asset pack {}",
                pack.id
            )));
        }
    }
    for version in &index.versions {
        if !pack_ids.contains(version.manifest.asset_pack.as_str()) {
            return Err(LauncherError::InvalidRemoteIndex(format!(
                "version {} references missing asset pack {}",
                version.manifest.version, version.manifest.asset_pack
            )));
        }
    }
    index
        .versions
        .sort_by(|left, right| right.manifest.version.cmp(&left.manifest.version));
    index
        .asset_packs
        .sort_by(|left, right| left.id.cmp(&right.id));
    Ok(index)
}

fn validate_version(entry: &RemoteVersion, index_url: &Url) -> Result<()> {
    if entry.package_size == 0 || entry.package_size > MAX_PACKAGE_BYTES {
        return Err(LauncherError::InvalidRemoteIndex(format!(
            "invalid package size for {}",
            entry.manifest.version
        )));
    }
    decode_hash(&entry.package_sha256)?;
    let package_url = Url::parse(&entry.package_url)
        .map_err(|error| LauncherError::InvalidRemoteIndex(error.to_string()))?;
    validate_same_origin(index_url, &package_url)
}

fn validate_asset_pack(pack: &RemoteAssetPack, index_url: &Url) -> Result<()> {
    if !valid_identifier(&pack.id) || pack.files.is_empty() || pack.files.len() > MAX_ASSET_FILES {
        return Err(LauncherError::InvalidRemoteIndex(format!(
            "invalid asset pack {}",
            pack.id
        )));
    }
    let mut paths = HashSet::new();
    let mut total = 0_u64;
    for file in &pack.files {
        validate_asset_file(file, index_url)?;
        if !paths.insert(file.path.as_str()) {
            return Err(LauncherError::InvalidRemoteIndex(format!(
                "duplicate asset path {}",
                file.path
            )));
        }
        total = total
            .checked_add(file.file_size)
            .ok_or(LauncherError::DownloadTooLarge)?;
    }
    if total > MAX_ASSET_PACK_BYTES {
        return Err(LauncherError::DownloadTooLarge);
    }
    Ok(())
}

fn validate_asset_file(file: &RemoteAssetFile, index_url: &Url) -> Result<()> {
    validate_relative_path(&file.path)?;
    if file.file_size == 0 || file.file_size > MAX_ASSET_FILE_BYTES {
        return Err(LauncherError::InvalidRemoteIndex(format!(
            "invalid size for asset {}",
            file.path
        )));
    }
    decode_hash(&file.file_sha256)?;
    let url = Url::parse(&file.file_url)
        .map_err(|error| LauncherError::InvalidRemoteIndex(error.to_string()))?;
    validate_same_origin(index_url, &url)
}

pub(crate) fn validate_relative_path(path: &str) -> Result<()> {
    let candidate = std::path::Path::new(path);
    if path.is_empty()
        || path.len() > 240
        || path.contains('\\')
        || path.split('/').any(|component| {
            component.is_empty()
                || !component
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
        || candidate.is_absolute()
        || candidate
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(LauncherError::InvalidAssetPath(path.to_owned()));
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

pub(super) fn decode_hash(encoded: &str) -> Result<[u8; 32]> {
    decode_fixed(encoded, "SHA-256")
}

fn decode_fixed<const N: usize>(encoded: &str, name: &str) -> Result<[u8; N]> {
    let bytes = hex::decode(encoded)
        .map_err(|error| LauncherError::InvalidRemoteIndex(error.to_string()))?;
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        LauncherError::InvalidRemoteIndex(format!("{name} has {} bytes, expected {N}", bytes.len()))
    })
}

pub(super) fn validate_same_origin(index: &Url, package: &Url) -> Result<()> {
    if package.scheme() != index.scheme()
        || package.host_str() != index.host_str()
        || package.port_or_known_default() != index.port_or_known_default()
    {
        return Err(LauncherError::InvalidRemoteIndex(
            "download URL must use the repository index origin".to_owned(),
        ));
    }
    Ok(())
}
