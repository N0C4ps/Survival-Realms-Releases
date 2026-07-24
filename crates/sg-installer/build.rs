use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is unavailable"),
    );
    let workspace = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("installer crate must be inside the workspace");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is unavailable"));
    let profile = env::var("PROFILE").unwrap_or_default();

    let distribution_launcher = workspace
        .join("target")
        .join("distribution")
        .join("survival-realms-launcher.exe");
    let debug_launcher = workspace
        .join("target")
        .join("debug")
        .join("survival-realms-launcher.exe");
    let launcher = if distribution_launcher.is_file() {
        distribution_launcher
    } else if profile != "distribution" && debug_launcher.is_file() {
        println!(
            "cargo:warning=Using the debug launcher payload. Build the distribution launcher before producing the public installer."
        );
        debug_launcher
    } else {
        panic!(
            "launcher payload not found; run `cargo build --profile distribution --package survival-realms-launcher` first"
        );
    };

    let version = workspace.join("versions").join("0.0.2.sg");
    if !version.is_file() {
        panic!(
            "version payload not found at {}; package Game 0.0.2 first",
            version.display()
        );
    }

    let assets_root = workspace.join("assets");
    let mut assets = Vec::new();
    collect_files(&assets_root, &assets_root, &mut assets);
    assets.sort_by(|left, right| left.0.cmp(&right.0));
    if assets.is_empty() {
        panic!("no assets found at {}", assets_root.display());
    }

    println!("cargo:rerun-if-changed={}", launcher.display());
    println!("cargo:rerun-if-changed={}", version.display());
    println!("cargo:rerun-if-changed={}", assets_root.display());

    let launcher_literal = rust_path_literal(&launcher);
    let version_literal = rust_path_literal(&version);
    let mut generated = format!(
        "pub static LAUNCHER: &[u8] = include_bytes!({launcher_literal});\n\
         pub static VERSION: &[u8] = include_bytes!({version_literal});\n\
         pub static ASSETS: &[(&str, &[u8])] = &[\n"
    );
    for (relative, absolute) in assets {
        generated.push_str(&format!(
            "    ({relative:?}, include_bytes!({})),\n",
            rust_path_literal(&absolute)
        ));
    }
    generated.push_str("];\n");

    fs::write(out_dir.join("installer_payload.rs"), generated)
        .expect("failed to generate installer payload");
}

fn collect_files(root: &Path, current: &Path, output: &mut Vec<(String, PathBuf)>) {
    let entries = fs::read_dir(current)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", current.display()));
    for entry in entries {
        let entry = entry.expect("failed to inspect asset entry");
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, output);
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .expect("asset must be below assets root")
                .to_string_lossy()
                .replace('\\', "/");
            output.push((relative, path));
        }
    }
}

fn rust_path_literal(path: &Path) -> String {
    format!("{:?}", path.to_string_lossy())
}
