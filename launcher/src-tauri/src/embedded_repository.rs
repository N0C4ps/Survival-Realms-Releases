use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use semver::Version;
use sg_format::{
    INDEX_SCHEMA_VERSION, PackageLimits, PackageReader, RemoteAssetPack, SignedVersionIndex,
    VersionIndex,
};
use sg_launcher_core::{
    AssetPackDownloadOutcome, DownloadOutcome, DownloadProgress, LauncherPaths, TrustedKeyring,
};
use sha2::{Digest, Sha256};
use url::Url;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/embedded_repository.rs"));
}

#[derive(Clone)]
pub(crate) struct EmbeddedRepository {
    index: VersionIndex,
}

impl EmbeddedRepository {
    pub(crate) fn public_key() -> [u8; 32] {
        generated::PUBLIC_KEY
    }

    pub(crate) fn open(keyring: &TrustedKeyring) -> Result<Self, String> {
        if generated::INDEX_BYTES.is_empty() {
            return Err("nenhuma versão foi embutida no launcher".to_owned());
        }
        let signed: SignedVersionIndex =
            serde_json::from_slice(generated::INDEX_BYTES).map_err(|error| error.to_string())?;
        let key_id = signed.signer_key_id().map_err(|error| error.to_string())?;
        let key = keyring
            .verifying_key(&key_id)
            .ok_or_else(|| format!("chave embutida desconhecida: {}", hex::encode(key_id)))?;
        signed.verify(key).map_err(|error| error.to_string())?;
        if signed.index.schema_version != INDEX_SCHEMA_VERSION {
            return Err(format!(
                "catálogo embutido usa schema incompatível {}",
                signed.index.schema_version
            ));
        }
        Ok(Self {
            index: signed.index,
        })
    }

    pub(crate) fn index(&self) -> VersionIndex {
        self.index.clone()
    }

    pub(crate) fn install<F>(
        &self,
        version: &Version,
        paths: &LauncherPaths,
        keyring: &TrustedKeyring,
        mut on_progress: F,
    ) -> Result<DownloadOutcome, String>
    where
        F: FnMut(DownloadProgress) -> bool,
    {
        let remote = self
            .index
            .find(version)
            .ok_or_else(|| format!("versão {version} não está embutida"))?;
        let pack = self
            .index
            .find_asset_pack(&remote.manifest.asset_pack)
            .ok_or_else(|| {
                format!(
                    "asset pack {} não está embutido",
                    remote.manifest.asset_pack
                )
            })?;
        let asset_size = pack
            .total_size()
            .ok_or_else(|| "asset pack excede o limite".to_owned())?;
        let total = remote
            .package_size
            .checked_add(asset_size)
            .ok_or_else(|| "conteúdo embutido excede o limite".to_owned())?;

        let package_bytes = embedded_file(&remote.package_url)?;
        verify_bytes(
            package_bytes,
            remote.package_size,
            &remote.package_sha256,
            "pacote do jogo",
        )?;
        if !on_progress(DownloadProgress {
            downloaded_bytes: remote.package_size,
            total_bytes: total,
        }) {
            return Err("instalação cancelada".to_owned());
        }
        let destination = paths
            .versions()
            .join(format!("{}.sg", remote.manifest.version));
        let already_present = write_immutable(&destination, package_bytes)?;
        validate_package(&destination, remote, keyring)?;

        let asset_outcome =
            install_assets(pack, paths, remote.package_size, total, &mut on_progress)?;
        Ok(DownloadOutcome {
            version: remote.manifest.version.clone(),
            package_path: destination,
            downloaded_bytes: if already_present && asset_outcome.already_present {
                0
            } else {
                total
            },
            already_present: already_present && asset_outcome.already_present,
        })
    }
}

fn validate_package(
    path: &Path,
    remote: &sg_format::RemoteVersion,
    keyring: &TrustedKeyring,
) -> Result<(), String> {
    let package =
        PackageReader::open(path, PackageLimits::default()).map_err(|error| error.to_string())?;
    if package.manifest() != &remote.manifest {
        return Err("manifesto do pacote difere do catálogo embutido".to_owned());
    }
    let key = keyring
        .verifying_key(&package.signer_key_id())
        .ok_or_else(|| "pacote assinado por chave desconhecida".to_owned())?;
    package
        .verify_signature(key)
        .and_then(|()| package.verify_payload())
        .map_err(|error| error.to_string())
}

fn install_assets<F>(
    pack: &RemoteAssetPack,
    paths: &LauncherPaths,
    package_size: u64,
    total_size: u64,
    on_progress: &mut F,
) -> Result<AssetPackDownloadOutcome, String>
where
    F: FnMut(DownloadProgress) -> bool,
{
    let destination = paths.asset_packs().join(&pack.id);
    let marker = serde_json::to_vec_pretty(pack).map_err(|error| error.to_string())?;
    if destination.join(".sg-pack.json").is_file()
        && fs::read(destination.join(".sg-pack.json")).ok().as_deref() == Some(marker.as_slice())
    {
        return Ok(AssetPackDownloadOutcome {
            id: pack.id.clone(),
            cache_path: destination,
            downloaded_bytes: 0,
            already_present: true,
        });
    }
    if destination.exists() {
        return Err(format!(
            "o cache do asset pack {} contém dados diferentes",
            pack.id
        ));
    }
    fs::create_dir_all(paths.asset_packs()).map_err(|error| error.to_string())?;
    let staging = temporary_path(&destination, "assets");
    fs::create_dir(&staging).map_err(|error| error.to_string())?;
    let result = (|| {
        let mut copied = 0_u64;
        for file in &pack.files {
            validate_relative_path(&file.path)?;
            let bytes = embedded_file(&file.file_url)?;
            verify_bytes(bytes, file.file_size, &file.file_sha256, &file.path)?;
            let target = staging.join(&file.path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            write_new(&target, bytes)?;
            copied += file.file_size;
            if !on_progress(DownloadProgress {
                downloaded_bytes: package_size + copied,
                total_bytes: total_size,
            }) {
                return Err("instalação cancelada".to_owned());
            }
        }
        write_new(&staging.join(".sg-pack.json"), &marker)?;
        fs::rename(&staging, &destination).map_err(|error| error.to_string())?;
        Ok(AssetPackDownloadOutcome {
            id: pack.id.clone(),
            cache_path: destination.clone(),
            downloaded_bytes: copied,
            already_present: false,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(staging);
    }
    result
}

fn embedded_file(url: &str) -> Result<&'static [u8], String> {
    let url = Url::parse(url).map_err(|error| error.to_string())?;
    let route = url.path().trim_start_matches('/');
    generated::FILES
        .iter()
        .find_map(|(candidate, bytes)| (*candidate == route).then_some(*bytes))
        .ok_or_else(|| format!("arquivo não foi embutido no launcher: {route}"))
}

fn verify_bytes(bytes: &[u8], size: u64, hash: &str, name: &str) -> Result<(), String> {
    if bytes.len() as u64 != size || hex::encode(Sha256::digest(bytes)) != hash {
        return Err(format!("conteúdo embutido inválido: {name}"));
    }
    Ok(())
}

fn write_immutable(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    if path.exists() {
        if fs::read(path).map_err(|error| error.to_string())? == bytes {
            return Ok(true);
        }
        return Err(format!(
            "arquivo instalado diverge da versão embutida: {}",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = temporary_path(path, "package");
    write_new(&temporary, bytes)?;
    fs::rename(&temporary, path).map_err(|error| error.to_string())?;
    Ok(false)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    let candidate = Path::new(path);
    if path.is_empty()
        || path.contains(['\\', ':'])
        || candidate.is_absolute()
        || candidate
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("caminho de asset inválido: {path}"));
    }
    Ok(())
}

fn temporary_path(destination: &Path, purpose: &str) -> PathBuf {
    destination.with_file_name(format!(
        ".{purpose}-{}-{}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

#[cfg(test)]
mod tests {
    use sg_launcher_core::AssetPackStore;

    use super::*;

    #[test]
    fn bundled_game_and_assets_install_without_network() {
        let temporary = tempfile::tempdir().unwrap();
        let paths = LauncherPaths::new(temporary.path());
        paths.initialize().unwrap();
        let mut keyring = TrustedKeyring::new();
        keyring
            .trust_bytes(&EmbeddedRepository::public_key())
            .unwrap();
        let repository = EmbeddedRepository::open(&keyring).unwrap();
        let version = Version::new(0, 0, 1);

        let outcome = repository
            .install(&version, &paths, &keyring, |_| true)
            .unwrap();
        let pack = repository
            .index()
            .find(&version)
            .unwrap()
            .manifest
            .asset_pack
            .clone();
        AssetPackStore::new(&paths).activate(&pack).unwrap();

        assert_eq!(outcome.version, version);
        assert!(outcome.package_path.is_file());
        assert!(paths.assets().join("texture/Grass_Block.png").is_file());
        assert!(paths.assets().join("particle/Grass_Particle.png").is_file());
        assert!(paths.saves().is_dir());
    }
}
