use std::{
    fs::{self, File},
    process::{Child, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    AssetPackStore, CatalogEntry, LauncherPaths, Result, SaveInspection, TrustedKeyring,
    VersionInstaller, save,
};

pub struct RunningGame {
    pub child: Child,
    pub log_path: std::path::PathBuf,
    pub save: SaveInspection,
}

pub struct GameLauncher<'a> {
    paths: &'a LauncherPaths,
    keyring: &'a TrustedKeyring,
}

impl<'a> GameLauncher<'a> {
    pub fn new(paths: &'a LauncherPaths, keyring: &'a TrustedKeyring) -> Self {
        Self { paths, keyring }
    }

    pub fn launch(&self, entry: &CatalogEntry) -> Result<RunningGame> {
        let installed = VersionInstaller::new(self.paths, self.keyring).prepare(entry)?;
        AssetPackStore::new(self.paths).activate(&installed.required_asset_pack)?;
        let save = save::inspect(&installed.executable, self.paths)?;
        fs::create_dir_all(self.paths.logs())?;
        let log_path =
            self.paths
                .logs()
                .join(format!("game-{}-{}.log", installed.version, timestamp()));
        let stdout = File::create(&log_path)?;
        let stderr = stdout.try_clone()?;
        let mut command = Command::new(&installed.executable);
        command
            .arg("--game-dir")
            .arg(self.paths.root())
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let child = command.spawn()?;
        Ok(RunningGame {
            child,
            log_path,
            save,
        })
    }
}

fn timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
