mod commands;
mod embedded_repository;
mod state;

pub fn run() {
    let backend = state::LauncherBackend::initialize()
        .unwrap_or_else(|error| panic!("failed to initialize launcher: {error}"));
    tauri::Builder::default()
        .manage(backend)
        .invoke_handler(tauri::generate_handler![
            commands::get_installation_status,
            commands::list_versions,
            commands::list_remote_versions,
            commands::download_version,
            commands::install_version,
            commands::launch_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Survival Realms Launcher");
}
