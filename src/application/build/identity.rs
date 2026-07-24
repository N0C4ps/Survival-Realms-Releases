use serde::Serialize;

use crate::application::core::{
    persistence::{MIN_SUPPORTED_SAVE_FORMAT, SAVE_FORMAT_VERSION},
    world::GENERATOR_VERSION,
};

use super::channel::ReleaseChannel;

const IDENTITY_SCHEMA_VERSION: u16 = 1;
const PROTOCOL_VERSION: u32 = 0;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct BuildIdentity {
    schema_version: u16,
    game_id: &'static str,
    display_name: &'static str,
    version: &'static str,
    channel: ReleaseChannel,
    platform: &'static str,
    architecture: &'static str,
    asset_pack: String,
    save_format: u32,
    minimum_save_format: u32,
    generator_version: u8,
    protocol_version: u32,
}

impl BuildIdentity {
    pub(crate) fn current() -> Self {
        Self {
            schema_version: IDENTITY_SCHEMA_VERSION,
            game_id: "survival-realms",
            display_name: "Survival Realms",
            version: env!("CARGO_PKG_VERSION"),
            channel: ReleaseChannel::current(),
            platform: std::env::consts::OS,
            architecture: std::env::consts::ARCH,
            asset_pack: option_env!("SG_ASSET_PACK")
                .map(str::to_owned)
                .unwrap_or_else(|| format!("assets-{}", env!("CARGO_PKG_VERSION"))),
            save_format: SAVE_FORMAT_VERSION,
            minimum_save_format: MIN_SUPPORTED_SAVE_FORMAT,
            generator_version: GENERATOR_VERSION,
            protocol_version: PROTOCOL_VERSION,
        }
    }

    pub(crate) fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_identity_has_independent_compatibility_versions() {
        let identity = BuildIdentity::current();

        assert_eq!(identity.game_id, "survival-realms");
        assert_eq!(identity.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(identity.save_format, SAVE_FORMAT_VERSION);
        assert_eq!(identity.generator_version, GENERATOR_VERSION);
        assert_eq!(identity.protocol_version, 0);
    }

    #[test]
    fn identity_serializes_as_launcher_readable_json() {
        let json = BuildIdentity::current().to_json().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["game_id"], "survival-realms");
        assert_eq!(value["schema_version"], IDENTITY_SCHEMA_VERSION);
        assert!(value["asset_pack"].as_str().unwrap().starts_with("assets-"));
    }
}
