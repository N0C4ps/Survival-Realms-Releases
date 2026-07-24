use super::{
    build::BuildIdentity,
    core::{persistence, runtime},
    launch::{LaunchMode, LaunchOptions},
    logging,
};

/// Starts and coordinates the application.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    logging::initialize();
    let options = LaunchOptions::from_process_arguments().map_err(std::io::Error::other)?;
    match options.mode {
        LaunchMode::VersionJson => {
            emit_metadata(
                options.metadata_output.as_deref(),
                &BuildIdentity::current().to_json()?,
            )?;
            return Ok(());
        }
        LaunchMode::InspectSave => {
            let inspection =
                persistence::inspect_save(&options.paths.level()).map_err(std::io::Error::other)?;
            emit_metadata(options.metadata_output.as_deref(), &inspection.to_json()?)?;
            return Ok(());
        }
        LaunchMode::Game => {}
    }
    if let Some(backup) = persistence::prepare_save_for_launch(&options.paths.level())
        .map_err(std::io::Error::other)?
    {
        tracing::info!(path = %backup.display(), "pre-migration level backup prepared");
    }
    tracing::info!(game_dir = %options.paths.root().display(), "game installation resolved");
    runtime::run(options.paths)?;
    Ok(())
}

fn emit_metadata(
    output: Option<&std::path::Path>,
    json: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(output) = output {
        std::fs::write(output, json)?;
    } else {
        println!("{json}");
    }
    Ok(())
}
