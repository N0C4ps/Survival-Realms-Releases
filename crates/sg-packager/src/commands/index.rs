use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use sg_format::{
    INDEX_SCHEMA_VERSION, PackageLimits, PackageReader, RemoteAssetFile, RemoteAssetPack,
    RemoteVersion, SignedVersionIndex, VersionIndex,
};
use sha2::{Digest, Sha256};
use url::Url;

use crate::{cli::IndexArgs, key_file, workspace};

pub(super) fn run(args: IndexArgs) -> Result<(), String> {
    let versions_dir = args
        .versions_dir
        .unwrap_or_else(|| workspace::root().join("versions"));
    let output = args
        .output
        .unwrap_or_else(|| versions_dir.join("index.json"));
    let base_url = parse_base_url(&args.base_url)?;
    let signing_key = key_file::read_private(&args.key)?;
    let mut versions = read_versions(&versions_dir, &base_url, &signing_key.verifying_key())?;
    if versions.is_empty() {
        return Err(format!(
            "no .sg packages found in {}",
            versions_dir.display()
        ));
    }
    versions.sort_by(|left, right| right.manifest.version.cmp(&left.manifest.version));
    let asset_pack_id = resolve_asset_pack_id(&versions, args.asset_pack_id)?;
    let assets_dir = args
        .assets_dir
        .unwrap_or_else(|| workspace::root().join("assets"));
    let asset_pack = read_asset_pack(&assets_dir, &asset_pack_id, &base_url)?;
    stage_asset_pack(
        &assets_dir,
        &asset_pack,
        output.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    let index = VersionIndex {
        schema_version: INDEX_SCHEMA_VERSION,
        generated_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs(),
        versions,
        asset_packs: vec![asset_pack],
    };
    let signed =
        SignedVersionIndex::sign(index, &signing_key).map_err(|error| error.to_string())?;
    let encoded = serde_json::to_vec_pretty(&signed).map_err(|error| error.to_string())?;
    write_atomically(&output, &encoded)?;

    let written: SignedVersionIndex =
        serde_json::from_slice(&fs::read(&output).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    written
        .verify(&signing_key.verifying_key())
        .map_err(|error| error.to_string())?;

    println!("index:    {}", output.display());
    println!("versions: {}", written.index.versions.len());
    println!("assets:   {}", written.index.asset_packs[0].files.len());
    println!("key id:   {}", written.signer_key_id);
    Ok(())
}

fn resolve_asset_pack_id(
    versions: &[RemoteVersion],
    explicit: Option<String>,
) -> Result<String, String> {
    let referenced = versions
        .iter()
        .map(|version| version.manifest.asset_pack.as_str())
        .collect::<HashSet<_>>();
    let id = match explicit {
        Some(id) => id,
        None if referenced.len() == 1 => referenced.iter().next().unwrap().to_string(),
        None => {
            return Err(
                "indexed versions reference multiple asset packs; pass --asset-pack-id and publish one compatible catalog at a time"
                    .to_owned(),
            );
        }
    };
    if referenced.iter().any(|referenced| *referenced != id) {
        return Err(format!(
            "asset pack {id} does not satisfy every indexed version"
        ));
    }
    Ok(id)
}

fn read_asset_pack(directory: &Path, id: &str, base_url: &Url) -> Result<RemoteAssetPack, String> {
    if !directory.is_dir() {
        return Err(format!(
            "asset directory not found: {}",
            directory.display()
        ));
    }
    let mut paths = Vec::new();
    collect_asset_files(directory, directory, &mut paths)?;
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    if paths.is_empty() {
        return Err(format!("asset directory is empty: {}", directory.display()));
    }
    let files = paths
        .into_iter()
        .map(|(relative, path)| {
            let size = fs::metadata(&path)
                .map_err(|error| error.to_string())?
                .len();
            if size == 0 {
                return Err(format!("asset is empty: {}", path.display()));
            }
            Ok(RemoteAssetFile {
                file_url: base_url
                    .join(&format!("assets/{id}/{relative}"))
                    .map_err(|error| error.to_string())?
                    .to_string(),
                path: relative,
                file_size: size,
                file_sha256: hex::encode(hash_file(&path)?),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RemoteAssetPack {
        id: id.to_owned(),
        files,
    })
}

fn collect_asset_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            return Err(format!(
                "symbolic links are not allowed in asset packs: {}",
                entry.path().display()
            ));
        }
        if file_type.is_dir() {
            collect_asset_files(root, &entry.path(), output)?;
        } else if file_type.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            if relative.split('/').any(|component| {
                component.is_empty()
                    || !component.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
            }) {
                return Err(format!(
                    "asset paths may only use ASCII letters, numbers, '.', '_' and '-': {relative}"
                ));
            }
            output.push((relative, entry.path()));
        }
    }
    Ok(())
}

fn stage_asset_pack(
    source: &Path,
    pack: &RemoteAssetPack,
    output_dir: &Path,
) -> Result<(), String> {
    let destination = output_dir.join("assets").join(&pack.id);
    if destination.exists() {
        for file in &pack.files {
            let path = destination.join(&file.path);
            if !path.is_file() || hex::encode(hash_file(&path)?) != file.file_sha256 {
                return Err(format!(
                    "published asset pack {} already exists with different contents; use a new asset-pack id",
                    pack.id
                ));
            }
        }
        return Ok(());
    }
    fs::create_dir_all(destination.parent().unwrap()).map_err(|error| error.to_string())?;
    let staging = temporary_path(&destination);
    let result = (|| {
        fs::create_dir(&staging).map_err(|error| error.to_string())?;
        for file in &pack.files {
            let target = staging.join(&file.path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::copy(source.join(&file.path), target).map_err(|error| error.to_string())?;
        }
        fs::rename(&staging, &destination).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn parse_base_url(raw: &str) -> Result<Url, String> {
    let mut url = Url::parse(raw).map_err(|error| error.to_string())?;
    let secure = matches!(url.scheme(), "https" | "embedded");
    let local_development = cfg!(debug_assertions)
        && url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "::1"));
    if !secure && !local_development {
        return Err("repository base URL must use HTTPS".to_owned());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("repository base URL cannot contain a query or fragment".to_owned());
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

fn read_versions(
    directory: &Path,
    base_url: &Url,
    verifying_key: &ed25519_dalek::VerifyingKey,
) -> Result<Vec<RemoteVersion>, String> {
    let mut packages = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("sg"))
        })
        .collect::<Vec<_>>();
    packages.sort();

    let mut seen = HashSet::new();
    packages
        .into_iter()
        .map(|path| {
            let package = PackageReader::open(&path, PackageLimits::default())
                .map_err(|error| format!("{}: {error}", path.display()))?;
            package
                .verify_signature(verifying_key)
                .and_then(|()| package.verify_payload())
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if !seen.insert(package.manifest().version.clone()) {
                return Err(format!(
                    "duplicate package version {}",
                    package.manifest().version
                ));
            }
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("package has a non-UTF-8 filename: {}", path.display()))?;
            Ok(RemoteVersion {
                manifest: package.manifest().clone(),
                package_url: base_url
                    .join(file_name)
                    .map_err(|error| error.to_string())?
                    .to_string(),
                package_size: fs::metadata(&path)
                    .map_err(|error| error.to_string())?
                    .len(),
                package_sha256: hex::encode(hash_file(&path)?),
            })
        })
        .collect()
}

fn hash_file(path: &Path) -> Result<[u8; 32], String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn write_atomically(output: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = temporary_path(output);
    let backup = temporary_path(output).with_extension("backup");
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        if output.exists() {
            fs::rename(output, &backup).map_err(|error| error.to_string())?;
        }
        if let Err(error) = fs::rename(&temporary, output) {
            if backup.exists() {
                let _ = fs::rename(&backup, output);
            }
            return Err(error.to_string());
        }
        if backup.exists() {
            fs::remove_file(&backup).map_err(|error| error.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
        if backup.exists() && !output.exists() {
            let _ = fs::rename(&backup, output);
        }
    }
    result
}

fn temporary_path(output: &Path) -> PathBuf {
    output.with_file_name(format!(
        ".{}.{}-{}.tmp",
        output.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_url_is_https_and_normalized_as_a_directory() {
        assert_eq!(
            parse_base_url("https://cdn.example/game").unwrap().as_str(),
            "https://cdn.example/game/"
        );
        assert!(parse_base_url("http://cdn.example/game/").is_err());
        assert!(parse_base_url("http://127.0.0.1:8787/").is_ok());
        assert!(parse_base_url("https://cdn.example/game/?token=x").is_err());
    }

    #[test]
    fn asset_pack_is_hashed_and_staged_with_its_public_layout() {
        let temporary = tempfile::tempdir().unwrap();
        let assets = temporary.path().join("source-assets");
        let publish = temporary.path().join("publish");
        fs::create_dir_all(assets.join("texture")).unwrap();
        fs::write(assets.join("texture/Stone.png"), b"png bytes").unwrap();
        let base = Url::parse("https://cdn.example/versions/").unwrap();

        let pack = read_asset_pack(&assets, "assets-test", &base).unwrap();
        assert_eq!(pack.files.len(), 1);
        assert_eq!(pack.files[0].path, "texture/Stone.png");
        assert_eq!(
            pack.files[0].file_url,
            "https://cdn.example/versions/assets/assets-test/texture/Stone.png"
        );

        stage_asset_pack(&assets, &pack, &publish).unwrap();
        assert_eq!(
            fs::read(publish.join("assets/assets-test/texture/Stone.png")).unwrap(),
            b"png bytes"
        );
        fs::write(
            publish.join("assets/assets-test/texture/Stone.png"),
            b"tampered",
        )
        .unwrap();
        assert!(stage_asset_pack(&assets, &pack, &publish).is_err());
    }
}
