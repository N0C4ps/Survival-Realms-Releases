use std::path::PathBuf;

use sg_launcher_core::{GithubRepository, LauncherPaths, TrustedKeyring};

use crate::embedded_repository::EmbeddedRepository;

pub(crate) struct LauncherBackend {
    pub paths: LauncherPaths,
    pub keyring: TrustedKeyring,
    pub embedded_repository: EmbeddedRepository,
    pub github_repository: GithubRepository,
}

impl LauncherBackend {
    pub fn initialize() -> Result<Self, String> {
        let paths = LauncherPaths::new(installation_root()?);
        paths.initialize().map_err(|error| error.to_string())?;
        let mut keyring = TrustedKeyring::new();
        keyring
            .trust_bytes(&EmbeddedRepository::public_key())
            .map_err(|error| error.to_string())?;
        let embedded_repository = EmbeddedRepository::open(&keyring)?;
        let github_repository = GithubRepository::official().map_err(|error| error.to_string())?;
        Ok(Self {
            paths,
            keyring,
            embedded_repository,
            github_repository,
        })
    }
}

fn installation_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("SG_INSTALL_DIR") {
        return Ok(PathBuf::from(root));
    }
    #[cfg(debug_assertions)]
    {
        Ok(workspace_root().join("launcher").join("dev-install"))
    }
    #[cfg(not(debug_assertions))]
    {
        std::env::current_exe()
            .map_err(|error| error.to_string())?
            .parent()
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| "launcher executable has no parent directory".to_owned())
    }
}

#[cfg(debug_assertions)]
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|launcher| launcher.parent())
        .expect("Tauri crate must remain under <workspace>/launcher")
        .to_path_buf()
}
