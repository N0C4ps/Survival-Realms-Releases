use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::Result;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherPaths {
    root: PathBuf,
}

impl LauncherPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn initialize(&self) -> Result<()> {
        for directory in [
            self.assets(),
            self.saves(),
            self.versions(),
            self.runtime(),
            self.launcher_data(),
            self.asset_packs(),
            self.logs(),
        ] {
            fs::create_dir_all(directory)?;
        }
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn saves(&self) -> PathBuf {
        self.root.join("saves")
    }

    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn settings(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn runtime(&self) -> PathBuf {
        self.root.join("runtime")
    }

    pub fn launcher_data(&self) -> PathBuf {
        self.root.join("launcher-data")
    }

    pub fn logs(&self) -> PathBuf {
        self.launcher_data().join("logs")
    }

    pub fn asset_packs(&self) -> PathBuf {
        self.launcher_data().join("asset-packs")
    }
}
