use semver::Version;
use serde::Serialize;
use sg_launcher_core::{
    AssetPackStore, DownloadOutcome, DownloadProgress, GithubCatalog, GithubVersion,
    InstalledVersion, VersionCatalog, VersionInstaller,
};
use tauri::{State, ipc::Channel};

use crate::state::LauncherBackend;

#[derive(Serialize)]
pub struct InstallationStatus {
    root: String,
    assets: String,
    saves: String,
    versions: String,
    runtime: String,
    trusted_keys: usize,
    repository_configured: bool,
    asset_packs: Vec<String>,
}

#[derive(Serialize)]
pub struct LaunchResponse {
    process_id: u32,
    log_path: String,
    save_status: String,
}

#[tauri::command]
pub fn get_installation_status(state: State<'_, LauncherBackend>) -> InstallationStatus {
    InstallationStatus {
        root: display(state.paths.root()),
        assets: display(&state.paths.assets()),
        saves: display(&state.paths.saves()),
        versions: display(&state.paths.versions()),
        runtime: display(&state.paths.runtime()),
        trusted_keys: state.keyring.len(),
        repository_configured: true,
        asset_packs: installed_asset_packs(&state.paths),
    }
}

fn installed_asset_packs(paths: &sg_launcher_core::LauncherPaths) -> Vec<String> {
    let mut packs = std::fs::read_dir(paths.asset_packs())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir() && entry.path().join(".sg-pack.json").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    packs.sort();
    packs
}

#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
pub enum DownloadEvent {
    Started {
        version: String,
        total_bytes: u64,
    },
    Progress(DownloadProgress),
    Finished {
        version: String,
        already_present: bool,
    },
}

#[tauri::command]
pub async fn list_remote_versions(
    state: State<'_, LauncherBackend>,
) -> Result<GithubCatalog, String> {
    match state.github_repository.fetch_catalog().await {
        Ok(catalog) if !catalog.versions.is_empty() => Ok(catalog),
        Ok(_) | Err(_) => Ok(embedded_catalog(&state.embedded_repository.index())),
    }
}

#[tauri::command]
pub async fn download_version(
    version: String,
    on_event: Channel<DownloadEvent>,
    state: State<'_, LauncherBackend>,
) -> Result<DownloadOutcome, String> {
    let version = parse_version(&version)?;
    let repository = state.github_repository.clone();
    let paths = state.paths.clone();
    let keyring = state.keyring.clone();
    let progress_channel = on_event.clone();
    let selected_version = version.clone();
    let mut started = false;
    let github_result = repository
        .install_version(&version, &paths, &keyring, move |progress| {
            if !started {
                if progress_channel
                    .send(DownloadEvent::Started {
                        version: selected_version.to_string(),
                        total_bytes: progress.total_bytes,
                    })
                    .is_err()
                {
                    return false;
                }
                started = true;
            }
            progress.downloaded_bytes == 0
                || progress_channel
                    .send(DownloadEvent::Progress(progress))
                    .is_ok()
        })
        .await;
    let outcome = match github_result {
        Ok(outcome) => outcome,
        Err(github_error) => install_from_embedded(
            &version,
            &paths,
            &keyring,
            &state.embedded_repository,
            &on_event,
        )
        .map_err(|embedded_error| {
            format!("GitHub: {github_error}; conteúdo offline: {embedded_error}")
        })?,
    };
    let _ = on_event.send(DownloadEvent::Finished {
        version: version.to_string(),
        already_present: outcome.already_present,
    });
    Ok(outcome)
}

fn embedded_catalog(index: &sg_launcher_core::VersionIndex) -> GithubCatalog {
    GithubCatalog {
        source: "embedded",
        versions: index
            .versions
            .iter()
            .map(|entry| {
                GithubVersion::offline(
                    entry.manifest.version.clone(),
                    entry.manifest.display_name.clone(),
                    entry.package_size,
                )
            })
            .collect(),
    }
}

fn install_from_embedded(
    version: &Version,
    paths: &sg_launcher_core::LauncherPaths,
    keyring: &sg_launcher_core::TrustedKeyring,
    repository: &crate::embedded_repository::EmbeddedRepository,
    on_event: &Channel<DownloadEvent>,
) -> Result<DownloadOutcome, String> {
    let index = repository.index();
    let remote = index
        .find(version)
        .cloned()
        .ok_or_else(|| format!("versão {version} não está no fallback offline"))?;
    let asset_pack = index
        .find_asset_pack(&remote.manifest.asset_pack)
        .cloned()
        .ok_or_else(|| format!("assets {} não encontrados", remote.manifest.asset_pack))?;
    let asset_bytes = asset_pack
        .total_size()
        .ok_or_else(|| "tamanho dos assets excede o limite".to_owned())?;
    let total_bytes = remote
        .package_size
        .checked_add(asset_bytes)
        .ok_or_else(|| "tamanho total excede o limite".to_owned())?;
    let _ = on_event.send(DownloadEvent::Started {
        version: version.to_string(),
        total_bytes,
    });
    let progress_channel = on_event.clone();
    let outcome = repository.install(version, paths, keyring, move |progress| {
        progress_channel
            .send(DownloadEvent::Progress(progress))
            .is_ok()
    })?;
    AssetPackStore::new(paths)
        .activate(&asset_pack.id)
        .map_err(|error| error.to_string())?;
    Ok(outcome)
}

#[tauri::command(async)]
pub fn list_versions(state: State<'_, LauncherBackend>) -> VersionCatalog {
    VersionCatalog::scan(&state.paths, &state.keyring)
}

#[tauri::command(async)]
pub fn install_version(
    version: String,
    state: State<'_, LauncherBackend>,
) -> Result<InstalledVersion, String> {
    let version = parse_version(&version)?;
    let catalog = VersionCatalog::scan(&state.paths, &state.keyring);
    let entry = catalog
        .find(&version)
        .ok_or_else(|| format!("version {version} is not available"))?;
    VersionInstaller::new(&state.paths, &state.keyring)
        .prepare(entry)
        .map_err(|error| error.to_string())
}

#[tauri::command(async)]
pub fn launch_version(
    version: String,
    state: State<'_, LauncherBackend>,
) -> Result<LaunchResponse, String> {
    let version = parse_version(&version)?;
    let catalog = VersionCatalog::scan(&state.paths, &state.keyring);
    let entry = catalog
        .find(&version)
        .ok_or_else(|| format!("version {version} is not available"))?;
    let running = sg_launcher_core::GameLauncher::new(&state.paths, &state.keyring)
        .launch(entry)
        .map_err(|error| error.to_string())?;
    Ok(LaunchResponse {
        process_id: running.child.id(),
        log_path: display(&running.log_path),
        save_status: running.save.status.to_string(),
    })
}

fn parse_version(version: &str) -> Result<Version, String> {
    Version::parse(version).map_err(|error| format!("invalid version '{version}': {error}"))
}

fn display(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedded_repository::EmbeddedRepository;
    use sg_launcher_core::{GithubRepository, LauncherPaths, TrustedKeyring};

    #[test]
    #[ignore = "uses the official public GitHub repository"]
    fn official_github_release_installs_incrementally() {
        tauri::async_runtime::block_on(async {
            let temporary = tempfile::tempdir().unwrap();
            let paths = LauncherPaths::new(temporary.path());
            paths.initialize().unwrap();
            let existing = paths.assets().join("texture/Grass_Block.png");
            std::fs::create_dir_all(existing.parent().unwrap()).unwrap();
            std::fs::write(&existing, b"custom texture").unwrap();

            let mut keyring = TrustedKeyring::new();
            keyring
                .trust_bytes(&EmbeddedRepository::public_key())
                .unwrap();
            let repository = GithubRepository::official().unwrap();
            let catalog = repository.fetch_catalog().await.unwrap();
            let version = Version::new(0, 0, 1);
            assert!(
                catalog
                    .versions
                    .iter()
                    .any(|entry| entry.version == version)
            );

            let outcome = repository
                .install_version(&version, &paths, &keyring, |_| true)
                .await
                .unwrap();
            assert!(outcome.package_path.is_file());
            assert!(paths.assets().join("particle/Grass_Particle.png").is_file());
            assert_eq!(std::fs::read(&existing).unwrap(), b"custom texture");

            let repeated = repository
                .install_version(&version, &paths, &keyring, |_| true)
                .await
                .unwrap();
            assert!(repeated.already_present);
            assert_eq!(repeated.downloaded_bytes, 0);
        });
    }
}
