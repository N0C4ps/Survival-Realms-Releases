use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use semver::Version;
use sg_format::{PackageBuilder, PackageLimits, PackageManifest, PackageReader, ReleaseChannel};

use crate::{cli::BuildArgs, identity::BuildIdentity, key_file, workspace};

const MINIMUM_LAUNCHER_VERSION: &str = "0.1.0";

pub(super) fn run(args: BuildArgs) -> Result<(), String> {
    if !args.skip_compile {
        compile_game(&args)?;
    }
    let executable = workspace::distribution_executable();
    if !executable.is_file() {
        return Err(format!(
            "distribution executable not found: {}; compile it first or remove --skip-compile",
            executable.display()
        ));
    }
    let identity = read_identity(&executable)?;
    validate_identity(&identity, &args)?;
    let manifest = manifest_from_identity(&identity)?;
    let output = args.output.unwrap_or_else(|| {
        workspace::root()
            .join("versions")
            .join(format!("{}.sg", identity.version))
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let executable_bytes = fs::read(&executable).map_err(|error| error.to_string())?;
    let signing_key = key_file::read_private(&args.key)?;
    PackageBuilder::new(&manifest, &executable_bytes)
        .compression_level(args.compression_level)
        .write_signed(&output, &signing_key)
        .map_err(|error| error.to_string())?;

    let package = PackageReader::open(&output, PackageLimits::default())
        .map_err(|error| error.to_string())?;
    package
        .verify_signature(&signing_key.verifying_key())
        .and_then(|()| package.verify_payload())
        .map_err(|error| error.to_string())?;

    println!("package: {}", output.display());
    println!("version: {} ({})", identity.version, identity.channel);
    println!(
        "size:    {} bytes",
        fs::metadata(&output).map_err(|e| e.to_string())?.len()
    );
    println!("key id:  {}", hex::encode(package.signer_key_id()));
    Ok(())
}

fn compile_game(args: &BuildArgs) -> Result<(), String> {
    println!("Compiling hardened Survival Realms build...");
    let mut command = Command::new("cargo");
    command
        .current_dir(workspace::root())
        .args([
            "build",
            "--profile",
            "distribution",
            "--package",
            "SurvivalRealms",
            "-j",
            &args.jobs.to_string(),
        ])
        .env("SG_RELEASE_CHANNEL", args.channel.as_str());
    if let Some(asset_pack) = &args.asset_pack {
        command.env("SG_ASSET_PACK", asset_pack);
    } else {
        command.env_remove("SG_ASSET_PACK");
    }
    let status = command.status().map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!("Cargo distribution build failed with {status}"));
    }
    Ok(())
}

fn read_identity(executable: &PathBuf) -> Result<BuildIdentity, String> {
    // Distribution builds are Windows GUI executables and therefore cannot be
    // trusted to keep an inherited stdout pipe. Ask the game to write its
    // metadata atomically to a temporary file instead.
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let metadata = executable.with_file_name(format!(
        ".sg-build-identity-{}-{nonce}.json",
        std::process::id()
    ));
    let output = Command::new(executable)
        .args(["--version-json", "--metadata-output"])
        .arg(&metadata)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let _ = fs::remove_file(&metadata);
        return Err(format!(
            "game identity command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let bytes = fs::read(&metadata).map_err(|error| {
        format!(
            "game did not write identity metadata to {}: {error}",
            metadata.display()
        )
    })?;
    let _ = fs::remove_file(&metadata);
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn validate_identity(identity: &BuildIdentity, args: &BuildArgs) -> Result<(), String> {
    if identity.channel != args.channel.as_str() {
        return Err(format!(
            "compiled executable channel is '{}', expected '{}'",
            identity.channel,
            args.channel.as_str()
        ));
    }
    if let Some(expected) = &args.asset_pack
        && &identity.asset_pack != expected
    {
        return Err(format!(
            "compiled executable asset pack is '{}', expected '{expected}'",
            identity.asset_pack
        ));
    }
    Ok(())
}

fn manifest_from_identity(identity: &BuildIdentity) -> Result<PackageManifest, String> {
    Ok(PackageManifest {
        build_identity_schema: identity.schema_version,
        game_id: identity.game_id.clone(),
        version: Version::parse(&identity.version).map_err(|error| error.to_string())?,
        display_name: release_display_name(identity),
        channel: parse_channel(&identity.channel)?,
        platform: identity.platform.clone(),
        architecture: identity.architecture.clone(),
        executable: if identity.platform == "windows" {
            "SurvivalRealms.exe".to_owned()
        } else {
            "SurvivalRealms".to_owned()
        },
        asset_pack: identity.asset_pack.clone(),
        minimum_save_format: identity.minimum_save_format,
        maximum_save_format: identity.save_format,
        generator_version: identity.generator_version,
        protocol_version: identity.protocol_version,
        minimum_launcher_version: Version::parse(MINIMUM_LAUNCHER_VERSION)
            .map_err(|error| error.to_string())?,
    })
}

fn release_display_name(identity: &BuildIdentity) -> String {
    match identity.version.as_str() {
        "0.0.1" => format!("{} 1One", identity.display_name),
        _ => format!("{} {}", identity.display_name, identity.version),
    }
}

fn parse_channel(channel: &str) -> Result<ReleaseChannel, String> {
    match channel {
        "development" => Ok(ReleaseChannel::Development),
        "snapshot" => Ok(ReleaseChannel::Snapshot),
        "release" => Ok(ReleaseChannel::Release),
        value => Err(format!("game returned an unknown release channel: {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_identity_maps_to_package_manifest_without_losing_versions() {
        let identity = BuildIdentity {
            schema_version: 1,
            game_id: "survival-realms".to_owned(),
            display_name: "Survival Realms".to_owned(),
            version: "0.2.0".to_owned(),
            channel: "snapshot".to_owned(),
            platform: "windows".to_owned(),
            architecture: "x86_64".to_owned(),
            asset_pack: "assets-0.2.0".to_owned(),
            save_format: 4,
            minimum_save_format: 2,
            generator_version: 3,
            protocol_version: 1,
        };
        let manifest = manifest_from_identity(&identity).unwrap();

        assert_eq!(manifest.version, Version::new(0, 2, 0));
        assert_eq!(manifest.channel, ReleaseChannel::Snapshot);
        assert_eq!(manifest.minimum_save_format, 2);
        assert_eq!(manifest.maximum_save_format, 4);
        assert_eq!(manifest.generator_version, 3);
        assert_eq!(manifest.protocol_version, 1);
    }
}
