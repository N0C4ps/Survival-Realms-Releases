use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GamePaths {
    root: PathBuf,
}

impl GamePaths {
    pub(crate) fn for_default_installation() -> Result<Self, String> {
        Self::new(default_root())
    }

    pub(crate) fn new(root: PathBuf) -> Result<Self, String> {
        if root.as_os_str().is_empty() {
            return Err("game directory cannot be empty".to_owned());
        }
        let root = if root.is_absolute() {
            root
        } else {
            env::current_dir()
                .map_err(|error| format!("failed to resolve game directory: {error}"))?
                .join(root)
        };
        Ok(Self { root })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub(crate) fn asset(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.assets_dir().join(relative)
    }

    pub(crate) fn saves_dir(&self) -> PathBuf {
        self.root.join("saves")
    }

    pub(crate) fn level(&self) -> PathBuf {
        self.saves_dir().join("world.level")
    }

    pub(crate) fn settings(&self) -> PathBuf {
        self.root.join("settings.json")
    }
}

#[cfg(debug_assertions)]
fn default_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(not(debug_assertions))]
fn default_root() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_root_controls_every_public_game_path() {
        let root = env::temp_dir().join("survival-realms-path-test");
        let paths = GamePaths::new(root.clone()).unwrap();

        assert_eq!(paths.root(), root);
        assert_eq!(paths.assets_dir(), root.join("assets"));
        assert_eq!(
            paths.asset("texture/Stone_Block.png"),
            root.join("assets/texture/Stone_Block.png")
        );
        assert_eq!(paths.saves_dir(), root.join("saves"));
        assert_eq!(paths.level(), root.join("saves/world.level"));
        assert_eq!(paths.settings(), root.join("settings.json"));
    }
}
