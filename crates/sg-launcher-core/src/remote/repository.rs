use std::time::Duration;

use futures_util::StreamExt;
use reqwest::{Client, redirect::Policy};
use sg_format::SignedVersionIndex;
use url::Url;

use crate::{LauncherError, Result, TrustedKeyring};

use super::{
    VersionIndex,
    index::{MAX_INDEX_BYTES, verify_index},
};

#[derive(Clone)]
pub struct RemoteRepository {
    client: Client,
    index_url: Url,
}

impl RemoteRepository {
    pub fn new(index_url: impl AsRef<str>) -> Result<Self> {
        let index_url = Url::parse(index_url.as_ref())
            .map_err(|error| LauncherError::UnsafeRepositoryUrl(error.to_string()))?;
        validate_repository_url(&index_url)?;
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(15 * 60))
            .user_agent(concat!(
                "SurvivalRealmsLauncher/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self { client, index_url })
    }

    pub fn index_url(&self) -> &Url {
        &self.index_url
    }

    pub async fn fetch_index(&self, keyring: &TrustedKeyring) -> Result<VersionIndex> {
        let response = self
            .client
            .get(self.index_url.clone())
            .send()
            .await?
            .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|size| size > MAX_INDEX_BYTES as u64)
        {
            return Err(LauncherError::DownloadTooLarge);
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if bytes.len() + chunk.len() > MAX_INDEX_BYTES {
                return Err(LauncherError::DownloadTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        let signed: SignedVersionIndex = serde_json::from_slice(&bytes)?;
        verify_index(signed, keyring, &self.index_url)
    }

    pub(crate) fn client(&self) -> &Client {
        &self.client
    }
}

fn validate_repository_url(url: &Url) -> Result<()> {
    if url.scheme() == "https" {
        return Ok(());
    }
    #[cfg(debug_assertions)]
    if url.scheme() == "http" && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "::1")) {
        return Ok(());
    }
    Err(LauncherError::UnsafeRepositoryUrl(url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_repository_requires_https() {
        assert!(RemoteRepository::new("https://updates.survivalrealms.example/index.json").is_ok());
        assert!(RemoteRepository::new("http://updates.survivalrealms.example/index.json").is_err());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn development_repository_may_use_loopback_http() {
        assert!(RemoteRepository::new("http://127.0.0.1:8080/index.json").is_ok());
    }

    #[cfg(debug_assertions)]
    #[tokio::test]
    async fn signed_index_and_package_download_round_trip() {
        use std::{
            fs,
            io::{Read, Write},
            net::TcpListener,
            thread,
        };

        use ed25519_dalek::SigningKey;
        use semver::Version;
        use sg_format::{
            INDEX_SCHEMA_VERSION, PackageBuilder, PackageManifest, ReleaseChannel, RemoteAssetFile,
            RemoteAssetPack, RemoteVersion, SignedVersionIndex, VersionIndex,
        };
        use sha2::{Digest, Sha256};

        use crate::LauncherPaths;

        let temporary = tempfile::tempdir().unwrap();
        let package_path = temporary.path().join("0.1.0.sg");
        let key = SigningKey::from_bytes(&[27; 32]);
        let manifest = PackageManifest {
            build_identity_schema: 1,
            game_id: "survival-realms".to_owned(),
            version: Version::new(0, 1, 0),
            display_name: "Survival Realms 0.1.0".to_owned(),
            channel: ReleaseChannel::Release,
            platform: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            executable: if cfg!(windows) {
                "SurvivalRealms.exe".to_owned()
            } else {
                "SurvivalRealms".to_owned()
            },
            asset_pack: "assets-0.1.0".to_owned(),
            minimum_save_format: 1,
            maximum_save_format: 3,
            generator_version: 1,
            protocol_version: 1,
            minimum_launcher_version: Version::new(0, 1, 0),
        };
        PackageBuilder::new(&manifest, b"test executable")
            .compression_level(1)
            .write_signed(&package_path, &key)
            .unwrap();
        let package_bytes = fs::read(&package_path).unwrap();
        let asset_bytes = b"test texture".to_vec();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let index_url = format!("http://{address}/index.json");
        let signed = SignedVersionIndex::sign(
            VersionIndex {
                schema_version: INDEX_SCHEMA_VERSION,
                generated_at: 1,
                versions: vec![RemoteVersion {
                    manifest: manifest.clone(),
                    package_url: format!("http://{address}/0.1.0.sg"),
                    package_size: package_bytes.len() as u64,
                    package_sha256: hex::encode(Sha256::digest(&package_bytes)),
                }],
                asset_packs: vec![RemoteAssetPack {
                    id: "assets-0.1.0".to_owned(),
                    files: vec![RemoteAssetFile {
                        path: "texture/test.png".to_owned(),
                        file_url: format!("http://{address}/assets/assets-0.1.0/texture/test.png"),
                        file_size: asset_bytes.len() as u64,
                        file_sha256: hex::encode(Sha256::digest(&asset_bytes)),
                    }],
                }],
            },
            &key,
        )
        .unwrap();
        let index_bytes = serde_json::to_vec(&signed).unwrap();
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 2048];
                let read = stream.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..read]);
                let body = if request.starts_with("GET /index.json ") {
                    &index_bytes
                } else if request.starts_with("GET /assets/") {
                    &asset_bytes
                } else {
                    &package_bytes
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(body).unwrap();
            }
        });

        let repository = RemoteRepository::new(index_url).unwrap();
        let mut keyring = TrustedKeyring::new();
        keyring.trust(key.verifying_key());
        let index = repository.fetch_index(&keyring).await.unwrap();
        let install = temporary.path().join("install");
        let paths = LauncherPaths::new(&install);
        let mut progress_events = 0;
        let outcome = repository
            .download_version(&index.versions[0], &paths, &keyring, |_| {
                progress_events += 1;
                true
            })
            .await
            .unwrap();
        let asset_outcome = repository
            .download_asset_pack(&index.asset_packs[0], &paths, |_| true)
            .await
            .unwrap();

        assert!(!outcome.already_present);
        assert!(outcome.package_path.is_file());
        assert!(!asset_outcome.already_present);
        assert_eq!(
            fs::read(asset_outcome.cache_path.join("texture/test.png")).unwrap(),
            b"test texture"
        );
        assert!(progress_events > 0);
        server.join().unwrap();
    }
}
