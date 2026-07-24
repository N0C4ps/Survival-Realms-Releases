use std::{fs, path::PathBuf, time::SystemTime};

use ed25519_dalek::SigningKey;
use semver::Version;

use crate::{
    INDEX_SCHEMA_VERSION, PackageBuilder, PackageError, PackageLimits, PackageManifest,
    PackageReader, ReleaseChannel, RemoteAssetPack, RemoteVersion, SignedVersionIndex,
    VersionIndex,
};

fn manifest() -> PackageManifest {
    PackageManifest {
        build_identity_schema: 1,
        game_id: "survival-realms".to_owned(),
        version: Version::new(0, 1, 0),
        display_name: "Survival Realms 0.1.0".to_owned(),
        channel: ReleaseChannel::Release,
        platform: "windows".to_owned(),
        architecture: "x86_64".to_owned(),
        executable: "SurvivalRealms.exe".to_owned(),
        asset_pack: "assets-0.1.0".to_owned(),
        minimum_save_format: 1,
        maximum_save_format: 3,
        generator_version: 2,
        protocol_version: 0,
        minimum_launcher_version: Version::new(0, 1, 0),
    }
}

#[test]
fn remote_index_signature_covers_every_version_field() {
    let key = SigningKey::from_bytes(&[11; 32]);
    let index = VersionIndex {
        schema_version: INDEX_SCHEMA_VERSION,
        generated_at: 123,
        versions: vec![RemoteVersion {
            manifest: manifest(),
            package_url: "https://cdn.example/0.1.0.sg".to_owned(),
            package_size: 4096,
            package_sha256: "ab".repeat(32),
        }],
        asset_packs: vec![RemoteAssetPack {
            id: "assets-0.1.0".to_owned(),
            files: Vec::new(),
        }],
    };
    let mut signed = SignedVersionIndex::sign(index, &key).unwrap();
    signed.verify(&key.verifying_key()).unwrap();

    signed.index.versions[0].package_size += 1;
    assert!(matches!(
        signed.verify(&key.verifying_key()),
        Err(PackageError::InvalidSignature)
    ));
}

fn temporary_directory(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("sg-format-{name}-{}-{unique}", std::process::id()))
}

#[test]
fn signed_package_round_trips_and_extracts_atomically() {
    let directory = temporary_directory("round-trip");
    fs::create_dir_all(&directory).unwrap();
    let package_path = directory.join("0.1.0.sg");
    let runtime = directory.join("runtime");
    let executable = b"MZ test Survival Realms executable";
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let manifest = manifest();

    PackageBuilder::new(&manifest, executable)
        .compression_level(1)
        .write_signed(&package_path, &signing_key)
        .unwrap();
    let package = PackageReader::open(&package_path, PackageLimits::default()).unwrap();

    assert_eq!(package.manifest(), &manifest);
    package
        .verify_signature(&signing_key.verifying_key())
        .unwrap();
    package.verify_payload().unwrap();
    let extracted = package
        .extract_executable(&runtime, &signing_key.verifying_key())
        .unwrap();
    assert_eq!(fs::read(extracted).unwrap(), executable);
    assert!(runtime.read_dir().unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".sg-part-")
    }));

    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn payload_tampering_is_detected_before_extraction() {
    let directory = temporary_directory("tampered");
    fs::create_dir_all(&directory).unwrap();
    let package_path = directory.join("tampered.sg");
    let signing_key = SigningKey::from_bytes(&[9; 32]);
    PackageBuilder::new(&manifest(), b"original executable")
        .compression_level(1)
        .write_signed(&package_path, &signing_key)
        .unwrap();

    let mut bytes = fs::read(&package_path).unwrap();
    *bytes.last_mut().unwrap() ^= 0x5a;
    fs::write(&package_path, bytes).unwrap();
    let package = PackageReader::open(&package_path, PackageLimits::default()).unwrap();

    assert!(matches!(
        package.verify_payload(),
        Err(PackageError::PayloadHashMismatch)
    ));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn package_rejects_a_different_trusted_signer() {
    let directory = temporary_directory("wrong-key");
    fs::create_dir_all(&directory).unwrap();
    let package_path = directory.join("wrong-key.sg");
    let signer = SigningKey::from_bytes(&[3; 32]);
    let other = SigningKey::from_bytes(&[4; 32]);
    PackageBuilder::new(&manifest(), b"executable")
        .compression_level(1)
        .write_signed(&package_path, &signer)
        .unwrap();
    let package = PackageReader::open(&package_path, PackageLimits::default()).unwrap();

    assert!(matches!(
        package.verify_signature(&other.verifying_key()),
        Err(PackageError::UnexpectedSigningKey)
    ));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn manifest_rejects_executable_path_traversal() {
    let mut invalid = manifest();
    invalid.executable = "../SurvivalRealms.exe".to_owned();
    let directory = temporary_directory("traversal");
    fs::create_dir_all(&directory).unwrap();
    let result = PackageBuilder::new(&invalid, b"executable").write_signed(
        directory.join("invalid.sg"),
        &SigningKey::from_bytes(&[1; 32]),
    );

    assert!(matches!(result, Err(PackageError::InvalidManifest(_))));
    fs::remove_dir_all(directory).unwrap();
}
