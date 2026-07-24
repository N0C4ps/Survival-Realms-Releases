use std::{
    fs,
    path::{Path, PathBuf},
};

fn main() {
    generate_embedded_repository().expect("failed to embed Survival Realms repository");
    tauri_build::build()
}

fn generate_embedded_repository() -> Result<(), String> {
    let workspace = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf();
    let versions = workspace.join("versions");
    let index = versions.join("index.json");
    let public_key = read_public_key(&workspace)?;
    let mut files = Vec::new();
    if versions.is_dir() {
        collect_files(&versions, &versions, &mut files)?;
    }
    files.retain(|(route, _)| route != "index.json");
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let index_expression = if index.is_file() {
        format!("include_bytes!(r#\"{}\"#)", index.display())
    } else {
        "&[]".to_owned()
    };
    let entries = files
        .iter()
        .map(|(route, path)| {
            format!(
                "    (r#\"{route}\"#, include_bytes!(r#\"{}\"#) as &[u8]),",
                path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let key = public_key
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let generated = format!(
        "pub static INDEX_BYTES: &[u8] = {index_expression};\n\
         pub const PUBLIC_KEY: [u8; 32] = [{key}];\n\
         pub static FILES: &[(&str, &[u8])] = &[\n{entries}\n];\n"
    );
    let output = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded_repository.rs");
    fs::write(output, generated).map_err(|error| error.to_string())?;
    println!("cargo:rerun-if-changed={}", versions.display());
    println!(
        "cargo:rerun-if-changed={}",
        workspace
            .join(".local-secrets/development.sgkey.pub")
            .display()
    );
    Ok(())
}

fn collect_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            collect_files(root, &entry.path(), output)?;
        } else {
            let route = entry
                .path()
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            output.push((route, entry.path()));
        }
    }
    Ok(())
}

fn read_public_key(workspace: &Path) -> Result<[u8; 32], String> {
    if let Ok(encoded) = std::env::var("SG_TRUSTED_PUBLIC_KEY_HEX") {
        let decoded = hex::decode(encoded).map_err(|error| error.to_string())?;
        return decoded
            .try_into()
            .map_err(|bytes: Vec<u8>| format!("public key has {} bytes", bytes.len()));
    }
    let path = workspace.join(".local-secrets/development.sgkey.pub");
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "cannot read development public key {}: {error}",
            path.display()
        )
    })?;
    if bytes.len() != 42 || &bytes[..8] != b"SGPUB\0\0\0" || bytes[8..10] != 1_u16.to_le_bytes() {
        return Err(format!(
            "invalid development public key: {}",
            path.display()
        ));
    }
    Ok(bytes[10..42].try_into().unwrap())
}
