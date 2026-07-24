use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions as SyncOpenOptions,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use futures_util::StreamExt;
use reqwest::{Client, Response, redirect::Policy};
use semver::Version;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sg_format::{PackageLimits, PackageReader};
use sha1::{Digest, Sha1};
use tokio::{fs, io::AsyncWriteExt};
use url::Url;

use crate::{
    DownloadOutcome, DownloadProgress, LauncherError, LauncherPaths, Result, TrustedKeyring,
    settings::SettingsIndex,
};

const OWNER: &str = "N0C4ps";
const REPOSITORY: &str = "Survival-Realms-Releases";
const DEFAULT_BRANCH: &str = "main";
const GAME_ID: &str = "survival-realms";
const API_ROOT: &str = "https://api.github.com";
const RAW_ROOT: &str = "https://raw.githubusercontent.com";
const RELEASES_PER_PAGE: usize = 100;
const MAX_RELEASE_PAGES: usize = 100;
const MAX_API_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_ASSET_INDEX_BYTES: usize = 2 * 1024 * 1024;
const MAX_SETTINGS_INDEX_BYTES: usize = 256 * 1024;
const MAX_PACKAGE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ASSET_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone)]
pub struct GithubRepository {
    client: Client,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GithubCatalog {
    pub source: &'static str,
    pub versions: Vec<GithubVersion>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GithubVersion {
    pub version: Version,
    pub display_name: String,
    pub tag_name: String,
    pub package_size: u64,
    pub prerelease: bool,
    #[serde(skip_serializing)]
    package_url: String,
}

impl GithubVersion {
    pub fn offline(version: Version, display_name: String, package_size: u64) -> Self {
        Self {
            tag_name: format!("v{version}"),
            version,
            display_name,
            package_size,
            prerelease: false,
            package_url: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    format: u32,
    version: Version,
    #[allow(dead_code)]
    display_name: String,
    assets: Vec<AssetIndexEntry>,
}

#[derive(Debug, Deserialize)]
struct AssetIndexEntry {
    path: String,
}

#[derive(Debug, Deserialize)]
struct GithubTree {
    truncated: bool,
    tree: Vec<GithubTreeEntry>,
}

#[derive(Debug, Deserialize)]
struct GithubTreeEntry {
    path: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
    size: Option<u64>,
}

#[derive(Clone, Debug)]
struct MissingAsset {
    relative_path: String,
    target: PathBuf,
    size: u64,
    git_sha: String,
}

impl GithubRepository {
    pub fn official() -> Result<Self> {
        let client = Client::builder()
            .redirect(Policy::limited(5))
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(15 * 60))
            .user_agent(concat!(
                "SurvivalRealmsLauncher/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self { client })
    }

    pub async fn fetch_catalog(&self) -> Result<GithubCatalog> {
        let mut versions = Vec::new();
        for page in 1..=MAX_RELEASE_PAGES {
            let url = format!(
                "{API_ROOT}/repos/{OWNER}/{REPOSITORY}/releases?per_page={RELEASES_PER_PAGE}&page={page}"
            );
            let releases: Vec<GithubRelease> =
                self.fetch_json(&url, MAX_API_RESPONSE_BYTES).await?;
            let page_size = releases.len();
            versions.extend(
                releases
                    .into_iter()
                    .filter(|release| !release.draft)
                    .flat_map(|release| {
                        let display_name = release
                            .name
                            .filter(|name| !name.trim().is_empty())
                            .unwrap_or_else(|| format!("Survival Realms {}", release.tag_name));
                        release.assets.into_iter().filter_map(move |asset| {
                            let version = package_version(&asset.name)?;
                            if asset.size == 0
                                || asset.size > MAX_PACKAGE_BYTES
                                || !valid_release_download_url(&asset.browser_download_url)
                            {
                                return None;
                            }
                            Some(GithubVersion {
                                version,
                                display_name: display_name.clone(),
                                tag_name: release.tag_name.clone(),
                                package_size: asset.size,
                                prerelease: release.prerelease,
                                package_url: asset.browser_download_url,
                            })
                        })
                    }),
            );
            if page_size < RELEASES_PER_PAGE {
                break;
            }
        }

        versions.sort_by(|left, right| right.version.cmp(&left.version));
        let mut seen = HashSet::new();
        versions.retain(|entry| seen.insert(entry.version.clone()));
        Ok(GithubCatalog {
            source: "github",
            versions,
        })
    }

    pub async fn install_version<F>(
        &self,
        version: &Version,
        paths: &LauncherPaths,
        keyring: &TrustedKeyring,
        mut on_progress: F,
    ) -> Result<DownloadOutcome>
    where
        F: FnMut(DownloadProgress) -> bool,
    {
        let catalog = self.fetch_catalog().await?;
        let remote = catalog
            .versions
            .into_iter()
            .find(|entry| &entry.version == version)
            .ok_or_else(|| LauncherError::MissingVersion(version.clone()))?;
        let settings_index = self.fetch_settings_index(version).await?;
        let missing_assets = self.missing_assets(version, paths).await?;
        let package_destination = paths.versions().join(format!("{version}.sg"));
        let package_present = if package_destination.is_file() {
            validate_package(&package_destination, version, keyring)?;
            true
        } else if package_destination.exists() {
            return Err(LauncherError::VersionFileConflict(package_destination));
        } else {
            false
        };
        let asset_bytes = missing_assets.iter().try_fold(0_u64, |total, asset| {
            total
                .checked_add(asset.size)
                .ok_or(LauncherError::DownloadTooLarge)
        })?;
        let total_bytes = asset_bytes
            .checked_add(if package_present {
                0
            } else {
                remote.package_size
            })
            .ok_or(LauncherError::DownloadTooLarge)?;
        if !on_progress(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes,
        }) {
            return Err(LauncherError::DownloadCancelled);
        }

        let mut downloaded_bytes = 0_u64;
        if !package_present {
            fs::create_dir_all(paths.versions()).await?;
            let temporary = temporary_path(&package_destination, "package");
            let result = self
                .download_package(
                    &remote,
                    &temporary,
                    downloaded_bytes,
                    total_bytes,
                    &mut on_progress,
                )
                .await;
            if let Err(error) = result {
                let _ = fs::remove_file(&temporary).await;
                return Err(error);
            }
            validate_package(&temporary, version, keyring)?;
            fs::rename(&temporary, &package_destination).await?;
            downloaded_bytes = downloaded_bytes
                .checked_add(remote.package_size)
                .ok_or(LauncherError::DownloadTooLarge)?;
        }

        for asset in &missing_assets {
            self.download_asset(asset, downloaded_bytes, total_bytes, &mut on_progress)
                .await?;
            downloaded_bytes = downloaded_bytes
                .checked_add(asset.size)
                .ok_or(LauncherError::DownloadTooLarge)?;
        }

        let settings_changed = if let Some(index) = settings_index {
            index.validate(version)?;
            index.merge_into(&paths.settings())?
        } else {
            false
        };

        Ok(DownloadOutcome {
            version: version.clone(),
            package_path: package_destination,
            downloaded_bytes,
            already_present: package_present && missing_assets.is_empty() && !settings_changed,
        })
    }

    async fn fetch_settings_index(&self, version: &Version) -> Result<Option<SettingsIndex>> {
        let url = format!(
            "{RAW_ROOT}/{OWNER}/{REPOSITORY}/{DEFAULT_BRANCH}/settings-indexes/{version}.json"
        );
        let parsed = Url::parse(&url)
            .map_err(|error| LauncherError::UnsafeRepositoryUrl(error.to_string()))?;
        let response = self.client.get(parsed).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response.error_for_status()?;
        let bytes = limited_bytes(response, MAX_SETTINGS_INDEX_BYTES).await?;
        Ok(Some(serde_json::from_slice(&bytes)?))
    }

    async fn missing_assets(
        &self,
        version: &Version,
        paths: &LauncherPaths,
    ) -> Result<Vec<MissingAsset>> {
        let index_url = format!(
            "{RAW_ROOT}/{OWNER}/{REPOSITORY}/{DEFAULT_BRANCH}/asset-indexes/{version}.json"
        );
        let index: AssetIndex = self.fetch_json(&index_url, MAX_ASSET_INDEX_BYTES).await?;
        if index.format != 1 || &index.version != version {
            return Err(LauncherError::InvalidRemoteIndex(format!(
                "asset index {version} has incompatible metadata"
            )));
        }
        let tree_url =
            format!("{API_ROOT}/repos/{OWNER}/{REPOSITORY}/git/trees/{DEFAULT_BRANCH}?recursive=1");
        let tree: GithubTree = self.fetch_json(&tree_url, MAX_API_RESPONSE_BYTES).await?;
        if tree.truncated {
            return Err(LauncherError::InvalidRemoteIndex(
                "GitHub asset tree is truncated".to_owned(),
            ));
        }
        let objects = tree
            .tree
            .into_iter()
            .filter(|entry| entry.kind == "blob" && entry.path.starts_with("asset-objects/"))
            .map(|entry| (entry.path.clone(), entry))
            .collect::<HashMap<_, _>>();
        let mut seen = HashSet::new();
        let mut missing = Vec::new();
        for entry in index.assets {
            validate_asset_path(&entry.path)?;
            if !seen.insert(entry.path.clone()) {
                return Err(LauncherError::InvalidRemoteIndex(format!(
                    "duplicate asset path: {}",
                    entry.path
                )));
            }
            let target = paths.assets().join(&entry.path);
            if target.is_file() {
                continue;
            }
            if target.exists() {
                return Err(LauncherError::InvalidAssetPath(entry.path));
            }
            let repository_path = format!("asset-objects/{}", entry.path);
            let object = objects.get(&repository_path).ok_or_else(|| {
                LauncherError::InvalidRemoteIndex(format!(
                    "asset is absent from GitHub: {}",
                    entry.path
                ))
            })?;
            let size = object.size.ok_or_else(|| {
                LauncherError::InvalidRemoteIndex(format!(
                    "asset has no declared size: {}",
                    entry.path
                ))
            })?;
            if size == 0 || size > MAX_ASSET_BYTES || !valid_git_sha(&object.sha) {
                return Err(LauncherError::InvalidRemoteIndex(format!(
                    "asset metadata is invalid: {}",
                    entry.path
                )));
            }
            missing.push(MissingAsset {
                relative_path: entry.path,
                target,
                size,
                git_sha: object.sha.clone(),
            });
        }
        Ok(missing)
    }

    async fn download_package<F>(
        &self,
        remote: &GithubVersion,
        destination: &Path,
        completed_bytes: u64,
        total_bytes: u64,
        on_progress: &mut F,
    ) -> Result<()>
    where
        F: FnMut(DownloadProgress) -> bool,
    {
        let response = self.get_https(&remote.package_url).await?;
        if response
            .content_length()
            .is_some_and(|length| length != remote.package_size)
        {
            return Err(LauncherError::DownloadTooLarge);
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .await?;
        let mut stream = response.bytes_stream();
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
            if !on_progress(DownloadProgress {
                downloaded_bytes: completed_bytes + downloaded,
                total_bytes,
            }) {
                return Err(LauncherError::DownloadCancelled);
            }
        }
        output.flush().await?;
        output.sync_all().await?;
        if downloaded != remote.package_size {
            return Err(LauncherError::DownloadTooLarge);
        }
        Ok(())
    }

    async fn download_asset<F>(
        &self,
        asset: &MissingAsset,
        completed_bytes: u64,
        total_bytes: u64,
        on_progress: &mut F,
    ) -> Result<()>
    where
        F: FnMut(DownloadProgress) -> bool,
    {
        if let Some(parent) = asset.target.parent() {
            fs::create_dir_all(parent).await?;
        }
        let url = format!(
            "{RAW_ROOT}/{OWNER}/{REPOSITORY}/{DEFAULT_BRANCH}/asset-objects/{}",
            asset.relative_path
        );
        let response = self.get_https(&url).await?;
        if response
            .content_length()
            .is_some_and(|length| length != asset.size)
        {
            return Err(LauncherError::DownloadHashMismatch);
        }
        let temporary = temporary_path(&asset.target, "asset");
        let result = async {
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .await?;
            let mut stream = response.bytes_stream();
            let mut downloaded = 0_u64;
            let mut hasher = Sha1::new();
            hasher.update(format!("blob {}\0", asset.size).as_bytes());
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                downloaded = downloaded
                    .checked_add(chunk.len() as u64)
                    .ok_or(LauncherError::DownloadTooLarge)?;
                if downloaded > asset.size || downloaded > MAX_ASSET_BYTES {
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
            if downloaded != asset.size || hex::encode(hasher.finalize()) != asset.git_sha {
                return Err(LauncherError::DownloadHashMismatch);
            }
            fs::rename(&temporary, &asset.target).await?;
            Ok::<(), LauncherError>(())
        }
        .await;
        if result.is_err() {
            let _ = fs::remove_file(&temporary).await;
        }
        result
    }

    async fn fetch_json<T: DeserializeOwned>(&self, url: &str, limit: usize) -> Result<T> {
        let response = self.get_https(url).await?;
        let bytes = limited_bytes(response, limit).await?;
        serde_json::from_slice(&bytes).map_err(LauncherError::from)
    }

    async fn get_https(&self, url: &str) -> Result<Response> {
        let parsed = Url::parse(url)
            .map_err(|error| LauncherError::UnsafeRepositoryUrl(error.to_string()))?;
        if parsed.scheme() != "https" {
            return Err(LauncherError::UnsafeRepositoryUrl(url.to_owned()));
        }
        let response = self.client.get(parsed).send().await?.error_for_status()?;
        if response.url().scheme() != "https" {
            return Err(LauncherError::UnsafeRepositoryUrl(
                response.url().to_string(),
            ));
        }
        Ok(response)
    }
}

async fn limited_bytes(response: Response, limit: usize) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(LauncherError::DownloadTooLarge);
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len() + chunk.len() > limit {
            return Err(LauncherError::DownloadTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn package_version(name: &str) -> Option<Version> {
    let stem = name.strip_suffix(".sg")?;
    Version::parse(stem).ok()
}

fn valid_release_download_url(raw: &str) -> bool {
    Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some("github.com")
            && url
                .path()
                .starts_with(&format!("/{OWNER}/{REPOSITORY}/releases/download/"))
    })
}

fn validate_asset_path(raw: &str) -> Result<()> {
    let path = Path::new(raw);
    let mut components = path.components();
    let root = components.next();
    if !matches!(
        root,
        Some(Component::Normal(value)) if value == "texture" || value == "particle"
    ) {
        return Err(LauncherError::InvalidAssetPath(raw.to_owned()));
    }
    let mut count = 1_usize;
    for component in components {
        let Component::Normal(value) = component else {
            return Err(LauncherError::InvalidAssetPath(raw.to_owned()));
        };
        let value = value.to_string_lossy();
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(LauncherError::InvalidAssetPath(raw.to_owned()));
        }
        count += 1;
    }
    if count < 2 || path.extension().is_none_or(|extension| extension != "png") {
        return Err(LauncherError::InvalidAssetPath(raw.to_owned()));
    }
    Ok(())
}

fn valid_git_sha(raw: &str) -> bool {
    raw.len() == 40 && raw.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_package(
    path: &Path,
    expected_version: &Version,
    keyring: &TrustedKeyring,
) -> Result<()> {
    let package = PackageReader::open(path, PackageLimits::default()).map_err(|error| {
        LauncherError::InvalidPackage {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })?;
    if package.manifest().game_id != GAME_ID || &package.manifest().version != expected_version {
        return Err(LauncherError::InvalidPackage {
            path: path.to_path_buf(),
            message: "package identity does not match the selected GitHub release".to_owned(),
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

fn temporary_path(destination: &Path, purpose: &str) -> PathBuf {
    destination.with_file_name(format!(
        ".{}.{}-{}-{}.tmp",
        destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        purpose,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

#[allow(dead_code)]
fn git_blob_sha(path: &Path) -> Result<String> {
    use std::io::Read;

    let mut file = SyncOpenOptions::new().read(true).open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {size}\0").as_bytes());
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_must_be_a_semantic_version() {
        assert_eq!(package_version("0.0.2.sg"), Some(Version::new(0, 0, 2)));
        assert_eq!(package_version("readme.txt"), None);
        assert_eq!(package_version("latest.sg"), None);
    }

    #[test]
    fn assets_are_confined_to_public_texture_and_particle_directories() {
        assert!(validate_asset_path("texture/Grass_Block.png").is_ok());
        assert!(validate_asset_path("particle/nested/Grass_Particle.png").is_ok());
        assert!(validate_asset_path("../saves/world.level").is_err());
        assert!(validate_asset_path("runtime/game.exe").is_err());
        assert!(validate_asset_path("texture/not-a-png.exe").is_err());
    }

    #[test]
    fn git_blob_hash_matches_git_empty_blob() {
        let temporary = tempfile::NamedTempFile::new().unwrap();
        assert_eq!(
            git_blob_sha(temporary.path()).unwrap(),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
    }
}
