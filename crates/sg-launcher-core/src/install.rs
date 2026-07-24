use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use semver::Version;
use serde::Serialize;
use sg_format::{PackageLimits, PackageReader};
use sha2::{Digest, Sha256};

use crate::{CatalogEntry, LauncherError, LauncherPaths, Result, TrustedKeyring};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InstalledVersion {
    pub version: Version,
    pub executable: PathBuf,
    pub required_asset_pack: String,
}

pub struct VersionInstaller<'a> {
    paths: &'a LauncherPaths,
    keyring: &'a TrustedKeyring,
}

impl<'a> VersionInstaller<'a> {
    pub fn new(paths: &'a LauncherPaths, keyring: &'a TrustedKeyring) -> Self {
        Self { paths, keyring }
    }

    pub fn prepare(&self, entry: &CatalogEntry) -> Result<InstalledVersion> {
        validate_platform(entry)?;
        let package = PackageReader::open(&entry.package_path, PackageLimits::default()).map_err(
            |error| LauncherError::InvalidPackage {
                path: entry.package_path.clone(),
                message: error.to_string(),
            },
        )?;
        if package.manifest() != &entry.manifest || package.signer_key_id() != entry.signer_key_id {
            return Err(LauncherError::InvalidPackage {
                path: entry.package_path.clone(),
                message: "package changed after catalog scan".to_owned(),
            });
        }
        let key = self
            .keyring
            .get(&entry.signer_key_id)
            .ok_or_else(|| LauncherError::UnknownSigningKey(hex_id(&entry.signer_key_id)))?;
        let runtime = self
            .paths
            .runtime()
            .join(entry.manifest.version.to_string())
            .join(package_runtime_id(&entry.package_path)?);
        let executable = package.extract_executable(runtime, key).map_err(|error| {
            LauncherError::InvalidPackage {
                path: entry.package_path.clone(),
                message: error.to_string(),
            }
        })?;
        Ok(InstalledVersion {
            version: entry.manifest.version.clone(),
            executable,
            required_asset_pack: entry.manifest.asset_pack.clone(),
        })
    }
}

fn package_runtime_id(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize())[..16].to_owned())
}

fn validate_platform(entry: &CatalogEntry) -> Result<()> {
    if entry.manifest.platform != std::env::consts::OS
        || entry.manifest.architecture != std::env::consts::ARCH
    {
        return Err(LauncherError::IncompatiblePlatform {
            version: entry.manifest.version.clone(),
            platform: entry.manifest.platform.clone(),
            architecture: entry.manifest.architecture.clone(),
            current_platform: std::env::consts::OS,
            current_architecture: std::env::consts::ARCH,
        });
    }
    Ok(())
}

fn hex_id(id: &[u8; 16]) -> String {
    id.iter().map(|byte| format!("{byte:02x}")).collect()
}
