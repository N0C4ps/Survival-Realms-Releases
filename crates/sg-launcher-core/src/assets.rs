use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{LauncherError, LauncherPaths, Result, remote::assets::PACK_MARKER};

pub struct AssetPackStore<'a> {
    paths: &'a LauncherPaths,
}

impl<'a> AssetPackStore<'a> {
    pub fn new(paths: &'a LauncherPaths) -> Self {
        Self { paths }
    }

    pub fn activate(&self, id: &str) -> Result<bool> {
        let cache = self.paths.asset_packs().join(id);
        if !cache.is_dir() {
            let active_marker = self.paths.assets().join(PACK_MARKER);
            if active_marker.exists() {
                let active: sg_format::RemoteAssetPack =
                    serde_json::from_slice(&fs::read(active_marker)?)?;
                if active.id == id {
                    return Ok(false);
                }
                return Err(LauncherError::MissingAssetPack(id.to_owned()));
            }
            if directory_has_public_assets(&self.paths.assets())? {
                return Ok(false);
            }
            return Err(LauncherError::MissingAssetPack(id.to_owned()));
        }
        let active_marker = self.paths.assets().join(PACK_MARKER);
        let cached_marker = fs::read(cache.join(PACK_MARKER))
            .map_err(|_| LauncherError::AssetPackConflict(cache.clone()))?;
        if fs::read(&active_marker).ok().as_deref() == Some(cached_marker.as_slice()) {
            return Ok(false);
        }

        let assets = self.paths.assets();
        let staging = sibling(&assets, "activate");
        let backup = sibling(&assets, "backup");
        copy_tree(&cache, &staging)?;
        let result = (|| {
            if assets.exists() {
                fs::rename(&assets, &backup)?;
            }
            if let Err(error) = fs::rename(&staging, &assets) {
                if backup.exists() {
                    let _ = fs::rename(&backup, &assets);
                }
                return Err(error.into());
            }
            if backup.exists() {
                fs::remove_dir_all(&backup)?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result.map(|()| true)
    }
}

fn directory_has_public_assets(path: &Path) -> Result<bool> {
    if !path.is_dir() {
        return Ok(false);
    }
    Ok(fs::read_dir(path)?.any(|entry| {
        entry
            .ok()
            .is_some_and(|entry| entry.file_name() != PACK_MARKER)
    }))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err(LauncherError::InvalidAssetPath(
                entry.path().to_string_lossy().into_owned(),
            ));
        }
    }
    Ok(())
}

fn sibling(destination: &Path, purpose: &str) -> PathBuf {
    destination.with_file_name(format!(
        ".assets-{purpose}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

#[cfg(test)]
mod tests {
    use sg_format::{RemoteAssetFile, RemoteAssetPack};

    use super::*;

    #[test]
    fn cached_pack_activation_replaces_assets_and_preserves_public_edits_until_switch() {
        let temporary = tempfile::tempdir().unwrap();
        let paths = LauncherPaths::new(temporary.path());
        paths.initialize().unwrap();
        fs::create_dir_all(paths.assets().join("texture")).unwrap();
        fs::write(paths.assets().join("texture/old.png"), b"old").unwrap();

        let pack = RemoteAssetPack {
            id: "assets-test".to_owned(),
            files: vec![RemoteAssetFile {
                path: "texture/new.png".to_owned(),
                file_url: "https://cdn.example/texture/new.png".to_owned(),
                file_size: 3,
                file_sha256: "00".repeat(32),
            }],
        };
        let cache = paths.asset_packs().join(&pack.id);
        fs::create_dir_all(cache.join("texture")).unwrap();
        fs::write(cache.join("texture/new.png"), b"new").unwrap();
        fs::write(cache.join(PACK_MARKER), serde_json::to_vec(&pack).unwrap()).unwrap();

        assert!(AssetPackStore::new(&paths).activate(&pack.id).unwrap());
        assert!(!paths.assets().join("texture/old.png").exists());
        assert_eq!(
            fs::read(paths.assets().join("texture/new.png")).unwrap(),
            b"new"
        );
        fs::write(paths.assets().join("texture/new.png"), b"modded").unwrap();
        assert!(!AssetPackStore::new(&paths).activate(&pack.id).unwrap());
        assert_eq!(
            fs::read(paths.assets().join("texture/new.png")).unwrap(),
            b"modded"
        );
    }
}
