use std::{env, ffi::OsString, path::PathBuf};

use crate::application::core::paths::GamePaths;

const GAME_DIR_ARGUMENT: &str = "--game-dir";
const INSPECT_SAVE_ARGUMENT: &str = "--inspect-save";
const METADATA_OUTPUT_ARGUMENT: &str = "--metadata-output";
const VERSION_JSON_ARGUMENT: &str = "--version-json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LaunchMode {
    Game,
    InspectSave,
    VersionJson,
}

pub(crate) struct LaunchOptions {
    pub(crate) mode: LaunchMode,
    pub(crate) paths: GamePaths,
    pub(crate) metadata_output: Option<PathBuf>,
}

impl LaunchOptions {
    pub(crate) fn from_process_arguments() -> Result<Self, String> {
        Self::parse(env::args_os().skip(1))
    }

    fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let mut game_dir = None;
        let mut metadata_output = None;
        let mut mode = LaunchMode::Game;

        while let Some(argument) = arguments.next() {
            if argument == GAME_DIR_ARGUMENT {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("{GAME_DIR_ARGUMENT} requires a directory"))?;
                if game_dir.replace(PathBuf::from(value)).is_some() {
                    return Err(format!("{GAME_DIR_ARGUMENT} was provided more than once"));
                }
            } else if argument == VERSION_JSON_ARGUMENT {
                if mode != LaunchMode::Game {
                    return Err("only one metadata command may be used at a time".to_owned());
                }
                mode = LaunchMode::VersionJson;
            } else if argument == INSPECT_SAVE_ARGUMENT {
                if mode != LaunchMode::Game {
                    return Err("only one metadata command may be used at a time".to_owned());
                }
                mode = LaunchMode::InspectSave;
            } else if argument == METADATA_OUTPUT_ARGUMENT {
                let value = arguments
                    .next()
                    .ok_or_else(|| format!("{METADATA_OUTPUT_ARGUMENT} requires a file"))?;
                if metadata_output.replace(PathBuf::from(value)).is_some() {
                    return Err(format!(
                        "{METADATA_OUTPUT_ARGUMENT} was provided more than once"
                    ));
                }
            } else {
                return Err(format!("unknown argument: {}", argument.to_string_lossy()));
            }
        }

        let paths = match game_dir {
            Some(root) => GamePaths::new(root)?,
            None => GamePaths::for_default_installation()?,
        };
        if metadata_output.is_some() && mode == LaunchMode::Game {
            return Err(format!(
                "{METADATA_OUTPUT_ARGUMENT} requires a metadata command"
            ));
        }
        Ok(Self {
            mode,
            paths,
            metadata_output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_launcher_game_directory_and_metadata_mode() {
        let root = env::temp_dir().join("survival-realms-launch-options");
        let options = LaunchOptions::parse([
            OsString::from(GAME_DIR_ARGUMENT),
            root.clone().into_os_string(),
            OsString::from(VERSION_JSON_ARGUMENT),
            OsString::from(METADATA_OUTPUT_ARGUMENT),
            root.join("metadata.json").into_os_string(),
        ])
        .unwrap();

        assert_eq!(options.mode, LaunchMode::VersionJson);
        assert_eq!(options.paths.root(), root);
        assert_eq!(
            options.metadata_output,
            Some(options.paths.root().join("metadata.json"))
        );
    }

    #[test]
    fn malformed_arguments_are_rejected() {
        assert!(LaunchOptions::parse([OsString::from(GAME_DIR_ARGUMENT)]).is_err());
        assert!(LaunchOptions::parse([OsString::from("--unexpected")]).is_err());
        assert!(
            LaunchOptions::parse([
                OsString::from(VERSION_JSON_ARGUMENT),
                OsString::from(INSPECT_SAVE_ARGUMENT),
            ])
            .is_err()
        );
    }
}
