#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use directories::BaseDirs;
use std::{
    env,
    ffi::OsStr,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};
use windows_sys::Win32::UI::{
    Shell::ShellExecuteW,
    WindowsAndMessaging::{
        IDOK, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_OKCANCEL,
        MB_YESNO, MessageBoxW, SW_SHOWNORMAL,
    },
};

mod payload {
    include!(concat!(env!("OUT_DIR"), "/installer_payload.rs"));
}

const PRODUCT_DIRECTORY: &str = ".survivalrealms";
const LAUNCHER_FILENAME: &str = "SurvivalRealmsLauncher.exe";
const VERSION_FILENAME: &str = "0.0.2.sg";
const DEFAULT_SETTINGS: &[u8] = br#"{
  "brightness": 75,
  "fov": 100,
  "mouse_sensitivity": 60,
  "render_distance": 8
}
"#;

fn main() -> ExitCode {
    let options = match Options::from_args() {
        Ok(options) => options,
        Err(message) => {
            show_error(&message);
            return ExitCode::FAILURE;
        }
    };

    if !options.silent {
        let prompt = format!(
            "Instalar o Survival Realms em:\n\n{}\n\nO jogo, o launcher e os assets serão instalados. Seus saves existentes serão preservados.",
            options.install_dir.display()
        );
        if message_box(
            &prompt,
            "Survival Realms Installer",
            MB_OKCANCEL | MB_ICONQUESTION,
        ) != IDOK
        {
            return ExitCode::SUCCESS;
        }
    }

    if let Err(error) = install_to(&options.install_dir) {
        if !options.silent {
            show_error(&format!(
                "Não foi possível instalar o Survival Realms.\n\n{error}"
            ));
        }
        return ExitCode::FAILURE;
    }

    let launcher = options.install_dir.join(LAUNCHER_FILENAME);
    let should_launch = if options.no_launch {
        false
    } else if options.silent {
        true
    } else {
        message_box(
            "Instalação concluída.\n\nDeseja abrir o launcher agora?",
            "Survival Realms Installer",
            MB_YESNO | MB_ICONINFORMATION,
        ) == IDYES
    };

    if should_launch && !launch(&launcher) {
        if !options.silent {
            show_error("A instalação terminou, mas o launcher não pôde ser aberto.");
        }
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[derive(Debug)]
struct Options {
    install_dir: PathBuf,
    silent: bool,
    no_launch: bool,
}

impl Options {
    fn from_args() -> Result<Self, String> {
        let default_dir = BaseDirs::new()
            .ok_or_else(|| "Não foi possível localizar a pasta de dados do Windows.".to_owned())?
            .config_dir()
            .join(PRODUCT_DIRECTORY);
        let mut install_dir = default_dir;
        let mut silent = false;
        let mut no_launch = false;
        let mut args = env::args_os().skip(1);

        while let Some(argument) = args.next() {
            match argument.to_string_lossy().as_ref() {
                "--silent" => silent = true,
                "--no-launch" => no_launch = true,
                "--install-dir" => {
                    install_dir = args
                        .next()
                        .map(PathBuf::from)
                        .ok_or_else(|| "--install-dir precisa de um caminho.".to_owned())?;
                }
                unknown => return Err(format!("Argumento desconhecido: {unknown}")),
            }
        }

        Ok(Self {
            install_dir,
            silent,
            no_launch,
        })
    }
}

fn install_to(root: &Path) -> io::Result<()> {
    let assets = root.join("assets");
    let saves = root.join("saves");
    let versions = root.join("versions");
    for directory in [
        assets.clone(),
        saves,
        versions.clone(),
        root.join("runtime"),
        root.join("launcher-data"),
        root.join("launcher-data/logs"),
        root.join("launcher-data/asset-packs"),
    ] {
        fs::create_dir_all(directory)?;
    }

    replace_if_different(&root.join(LAUNCHER_FILENAME), payload::LAUNCHER)?;
    replace_if_different(&versions.join(VERSION_FILENAME), payload::VERSION)?;
    let settings = root.join("settings.json");
    if !settings.exists() {
        write_new(&settings, DEFAULT_SETTINGS)?;
    }

    for (relative, bytes) in payload::ASSETS {
        let destination = safe_asset_path(&assets, relative)?;
        if !destination.exists() {
            write_new(&destination, bytes)?;
        }
    }

    Ok(())
}

fn safe_asset_path(root: &Path, relative: &str) -> io::Result<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid embedded asset path",
        ));
    }
    Ok(root.join(relative))
}

fn replace_if_different(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    if fs::read(destination).is_ok_and(|current| current == bytes) {
        return Ok(());
    }

    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = temporary_sibling(destination);
    write_exact(&temporary, bytes)?;

    let backup = destination.with_extension("installer-backup");
    if destination.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(destination, &backup)?;
    }
    match fs::rename(&temporary, destination) {
        Ok(()) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            if backup.exists() {
                let _ = fs::rename(&backup, destination);
            }
            let _ = fs::remove_file(temporary);
            Err(error)
        }
    }
}

fn write_new(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(destination)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn write_exact(destination: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(destination)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn temporary_sibling(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .unwrap_or_else(|| OsStr::new("payload"))
        .to_string_lossy();
    destination.with_file_name(format!(".{name}.install-{}.tmp", std::process::id()))
}

fn launch(executable: &Path) -> bool {
    let executable = wide(executable.as_os_str());
    let operation = wide(OsStr::new("open"));
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            executable.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    result as isize > 32
}

fn show_error(message: &str) {
    message_box(message, "Survival Realms Installer", MB_OK | MB_ICONERROR);
}

fn message_box(message: &str, title: &str, style: u32) -> i32 {
    let message = wide(OsStr::new(message));
    let title = wide(OsStr::new(title));
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            style,
        )
    }
}

fn wide(value: &OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_expected_layout_and_preserves_user_data() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let root = temporary.path();
        let save = root.join("saves").join("world.level");
        let customized_asset = root.join("assets").join(payload::ASSETS[0].0);
        fs::create_dir_all(save.parent().unwrap()).unwrap();
        fs::create_dir_all(customized_asset.parent().unwrap()).unwrap();
        fs::write(&save, b"my world").unwrap();
        fs::write(&customized_asset, b"custom texture").unwrap();

        install_to(root).unwrap();
        install_to(root).unwrap();

        assert_eq!(fs::read(save).unwrap(), b"my world");
        assert_eq!(fs::read(customized_asset).unwrap(), b"custom texture");
        assert_eq!(
            fs::read(root.join(LAUNCHER_FILENAME)).unwrap(),
            payload::LAUNCHER
        );
        assert_eq!(
            fs::read(root.join("versions").join(VERSION_FILENAME)).unwrap(),
            payload::VERSION
        );
        assert_eq!(
            fs::read(root.join("settings.json")).unwrap(),
            DEFAULT_SETTINGS
        );
        assert!(root.join("runtime").is_dir());
        assert!(root.join("launcher-data/logs").is_dir());
        assert!(root.join("launcher-data/asset-packs").is_dir());
        for (relative, _) in payload::ASSETS {
            assert!(root.join("assets").join(relative).is_file());
        }
    }

    #[test]
    fn rejects_asset_parent_traversal() {
        let root = Path::new("assets");
        assert!(safe_asset_path(root, "../escape").is_err());
    }
}
