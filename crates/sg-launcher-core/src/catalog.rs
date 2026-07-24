use std::{collections::HashSet, fs, path::PathBuf};

use semver::Version;
use serde::Serialize;
use sg_format::{KeyId, PackageLimits, PackageManifest, PackageReader};

use crate::{LauncherPaths, TrustedKeyring};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CatalogEntry {
    pub package_path: PathBuf,
    pub manifest: PackageManifest,
    pub signer_key_id: KeyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RejectedPackage {
    pub package_path: PathBuf,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct VersionCatalog {
    pub versions: Vec<CatalogEntry>,
    pub rejected: Vec<RejectedPackage>,
}

impl VersionCatalog {
    pub fn scan(paths: &LauncherPaths, keyring: &TrustedKeyring) -> Self {
        let mut catalog = Self::default();
        let entries = match fs::read_dir(paths.versions()) {
            Ok(entries) => entries,
            Err(error) => {
                catalog.rejected.push(RejectedPackage {
                    package_path: paths.versions(),
                    reason: error.to_string(),
                });
                return catalog;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !entry.file_type().is_ok_and(|kind| kind.is_file())
                || path.extension().and_then(|value| value.to_str()) != Some("sg")
            {
                continue;
            }
            match inspect_package(path.clone(), keyring) {
                Ok(version) => catalog.versions.push(version),
                Err(reason) => catalog.rejected.push(RejectedPackage {
                    package_path: path,
                    reason,
                }),
            }
        }
        reject_duplicate_versions(&mut catalog);
        catalog
            .versions
            .sort_by(|left, right| right.manifest.version.cmp(&left.manifest.version));
        catalog.rejected.sort_by(|left, right| {
            left.package_path
                .to_string_lossy()
                .cmp(&right.package_path.to_string_lossy())
        });
        catalog
    }

    pub fn find(&self, version: &Version) -> Option<&CatalogEntry> {
        self.versions
            .iter()
            .find(|entry| &entry.manifest.version == version)
    }
}

fn inspect_package(
    path: PathBuf,
    keyring: &TrustedKeyring,
) -> std::result::Result<CatalogEntry, String> {
    let package =
        PackageReader::open(&path, PackageLimits::default()).map_err(|error| error.to_string())?;
    if package.manifest().game_id != "survival-realms" {
        return Err(format!(
            "package belongs to unknown game '{}'",
            package.manifest().game_id
        ));
    }
    if package.manifest().build_identity_schema != 1 {
        return Err(format!(
            "unsupported build identity schema {}",
            package.manifest().build_identity_schema
        ));
    }
    let launcher_version = Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("launcher-core package version must be semantic");
    if package.manifest().minimum_launcher_version > launcher_version {
        return Err(format!(
            "requires launcher {}, current launcher is {}",
            package.manifest().minimum_launcher_version,
            launcher_version
        ));
    }
    let signer = package.signer_key_id();
    let trusted = keyring
        .get(&signer)
        .ok_or_else(|| format!("unknown signing key {}", hex_id(&signer)))?;
    package
        .verify_signature(trusted)
        .map_err(|error| error.to_string())?;
    Ok(CatalogEntry {
        package_path: path,
        manifest: package.manifest().clone(),
        signer_key_id: signer,
    })
}

fn reject_duplicate_versions(catalog: &mut VersionCatalog) {
    let mut seen = HashSet::new();
    let mut duplicates = HashSet::new();
    for entry in &catalog.versions {
        if !seen.insert(entry.manifest.version.clone()) {
            duplicates.insert(entry.manifest.version.clone());
        }
    }
    if duplicates.is_empty() {
        return;
    }
    let mut retained = Vec::new();
    for entry in catalog.versions.drain(..) {
        if duplicates.contains(&entry.manifest.version) {
            catalog.rejected.push(RejectedPackage {
                package_path: entry.package_path,
                reason: format!("duplicate version {}", entry.manifest.version),
            });
        } else {
            retained.push(entry);
        }
    }
    catalog.versions = retained;
}

fn hex_id(id: &KeyId) -> String {
    id.iter().map(|byte| format!("{byte:02x}")).collect()
}
