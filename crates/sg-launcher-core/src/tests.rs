use std::fs;

use ed25519_dalek::SigningKey;
use semver::Version;
use sg_format::{PackageBuilder, PackageManifest, ReleaseChannel};
use tempfile::TempDir;

use crate::{LauncherPaths, TrustedKeyring, VersionCatalog, VersionInstaller};

fn manifest(version: Version) -> PackageManifest {
    PackageManifest {
        build_identity_schema: 1,
        game_id: "survival-realms".to_owned(),
        version: version.clone(),
        display_name: format!("Survival Realms {version}"),
        channel: ReleaseChannel::Release,
        platform: std::env::consts::OS.to_owned(),
        architecture: std::env::consts::ARCH.to_owned(),
        executable: if cfg!(windows) {
            "SurvivalRealms.exe".to_owned()
        } else {
            "SurvivalRealms".to_owned()
        },
        asset_pack: format!("assets-{version}"),
        minimum_save_format: 1,
        maximum_save_format: 3,
        generator_version: 2,
        protocol_version: 0,
        minimum_launcher_version: Version::new(0, 1, 0),
    }
}

fn installation() -> (TempDir, LauncherPaths) {
    let temporary = tempfile::tempdir().unwrap();
    let paths = LauncherPaths::new(temporary.path());
    paths.initialize().unwrap();
    (temporary, paths)
}

#[test]
fn installation_layout_contains_every_public_and_internal_directory() {
    let (_temporary, paths) = installation();

    for directory in [
        paths.assets(),
        paths.saves(),
        paths.versions(),
        paths.runtime(),
        paths.launcher_data(),
        paths.asset_packs(),
        paths.logs(),
    ] {
        assert!(directory.is_dir(), "missing {}", directory.display());
    }
}

#[test]
fn catalog_validates_signatures_and_sorts_versions_newest_first() {
    let (_temporary, paths) = installation();
    let signer = SigningKey::from_bytes(&[11; 32]);
    for version in [Version::new(0, 1, 0), Version::new(0, 2, 0)] {
        PackageBuilder::new(&manifest(version.clone()), b"test executable")
            .compression_level(1)
            .write_signed(paths.versions().join(format!("{version}.sg")), &signer)
            .unwrap();
    }
    fs::write(paths.versions().join("notes.txt"), b"ignored").unwrap();
    let mut keyring = TrustedKeyring::new();
    keyring.trust(signer.verifying_key());

    let catalog = VersionCatalog::scan(&paths, &keyring);

    assert!(catalog.rejected.is_empty());
    assert_eq!(catalog.versions.len(), 2);
    assert_eq!(catalog.versions[0].manifest.version, Version::new(0, 2, 0));
    assert_eq!(catalog.versions[1].manifest.version, Version::new(0, 1, 0));
}

#[test]
fn unknown_signer_is_visible_as_a_rejected_package() {
    let (_temporary, paths) = installation();
    let signer = SigningKey::from_bytes(&[12; 32]);
    PackageBuilder::new(&manifest(Version::new(1, 0, 0)), b"test executable")
        .compression_level(1)
        .write_signed(paths.versions().join("1.0.0.sg"), &signer)
        .unwrap();

    let catalog = VersionCatalog::scan(&paths, &TrustedKeyring::new());

    assert!(catalog.versions.is_empty());
    assert_eq!(catalog.rejected.len(), 1);
    assert!(catalog.rejected[0].reason.contains("unknown signing key"));
}

#[test]
fn installer_extracts_and_revalidates_an_existing_runtime() {
    let (_temporary, paths) = installation();
    let signer = SigningKey::from_bytes(&[13; 32]);
    PackageBuilder::new(&manifest(Version::new(0, 3, 0)), b"runtime executable")
        .compression_level(1)
        .write_signed(paths.versions().join("0.3.0.sg"), &signer)
        .unwrap();
    let mut keyring = TrustedKeyring::new();
    keyring.trust(signer.verifying_key());
    let catalog = VersionCatalog::scan(&paths, &keyring);
    let installer = VersionInstaller::new(&paths, &keyring);

    let first = installer.prepare(&catalog.versions[0]).unwrap();
    let second = installer.prepare(&catalog.versions[0]).unwrap();

    assert_eq!(first, second);
    assert_eq!(fs::read(first.executable).unwrap(), b"runtime executable");
}

#[test]
fn duplicate_versions_are_all_rejected_instead_of_being_ambiguous() {
    let (_temporary, paths) = installation();
    let signer = SigningKey::from_bytes(&[14; 32]);
    let version = Version::new(0, 4, 0);
    for name in ["first.sg", "second.sg"] {
        PackageBuilder::new(&manifest(version.clone()), b"runtime executable")
            .compression_level(1)
            .write_signed(paths.versions().join(name), &signer)
            .unwrap();
    }
    let mut keyring = TrustedKeyring::new();
    keyring.trust(signer.verifying_key());

    let catalog = VersionCatalog::scan(&paths, &keyring);

    assert!(catalog.versions.is_empty());
    assert_eq!(catalog.rejected.len(), 2);
}

#[test]
fn package_requiring_a_newer_launcher_is_rejected() {
    let (_temporary, paths) = installation();
    let signer = SigningKey::from_bytes(&[15; 32]);
    let mut future = manifest(Version::new(0, 5, 0));
    future.minimum_launcher_version = Version::new(999, 0, 0);
    PackageBuilder::new(&future, b"runtime executable")
        .compression_level(1)
        .write_signed(paths.versions().join("future.sg"), &signer)
        .unwrap();
    let mut keyring = TrustedKeyring::new();
    keyring.trust(signer.verifying_key());

    let catalog = VersionCatalog::scan(&paths, &keyring);

    assert!(catalog.versions.is_empty());
    assert!(catalog.rejected[0].reason.contains("requires launcher"));
}

#[cfg(windows)]
#[test]
#[ignore = "uses the packaged distribution artifact"]
fn packaged_1one_metadata_round_trip_uses_a_private_file() {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap()
        .to_path_buf();
    let (_temporary, paths) = installation();
    fs::copy(
        workspace.join("versions/0.0.1.sg"),
        paths.versions().join("0.0.1.sg"),
    )
    .unwrap();
    let mut keyring = TrustedKeyring::new();
    let encoded_key = fs::read(workspace.join(".local-secrets/development.sgkey.pub")).unwrap();
    let public_key: [u8; 32] = encoded_key[encoded_key.len() - 32..].try_into().unwrap();
    keyring.trust_bytes(&public_key).unwrap();
    let catalog = VersionCatalog::scan(&paths, &keyring);
    assert!(catalog.rejected.is_empty());
    assert_eq!(
        catalog.versions[0].manifest.display_name,
        "Survival Realms 1One"
    );

    let installed = VersionInstaller::new(&paths, &keyring)
        .prepare(&catalog.versions[0])
        .unwrap();
    let inspection = crate::save::inspect(&installed.executable, &paths).unwrap();

    assert_eq!(inspection.status.to_string(), "missing");
    assert!(
        installed
            .executable
            .parent()
            .unwrap()
            .parent()
            .is_some_and(|parent| parent.ends_with("0.0.1"))
    );
}
