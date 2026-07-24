use std::path::PathBuf;

pub(crate) fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("sg-packager must remain inside <workspace>/crates")
        .to_path_buf()
}

pub(crate) fn distribution_executable() -> PathBuf {
    root()
        .join("target")
        .join("distribution")
        .join(if cfg!(windows) {
            "SurvivalRealms.exe"
        } else {
            "SurvivalRealms"
        })
}
