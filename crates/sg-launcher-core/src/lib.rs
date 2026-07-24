mod assets;
mod catalog;
mod error;
mod github;
mod install;
mod keyring;
mod launch;
mod paths;
mod remote;
mod save;
mod settings;

pub use catalog::{CatalogEntry, RejectedPackage, VersionCatalog};
pub use error::{LauncherError, Result};
pub use github::{GithubCatalog, GithubRepository, GithubVersion};
pub use install::{InstalledVersion, VersionInstaller};
pub use keyring::TrustedKeyring;
pub use launch::{GameLauncher, RunningGame};
pub use paths::LauncherPaths;
pub use remote::{
    AssetPackDownloadOutcome, DownloadOutcome, DownloadProgress, RemoteAssetFile, RemoteAssetPack,
    RemoteRepository, RemoteVersion, VersionIndex,
};
pub use save::{SaveInspection, SaveStatus};

#[cfg(test)]
mod tests;
pub use assets::AssetPackStore;
