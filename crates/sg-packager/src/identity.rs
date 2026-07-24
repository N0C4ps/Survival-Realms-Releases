use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct BuildIdentity {
    pub schema_version: u16,
    pub game_id: String,
    pub display_name: String,
    pub version: String,
    pub channel: String,
    pub platform: String,
    pub architecture: String,
    pub asset_pack: String,
    pub save_format: u32,
    pub minimum_save_format: u32,
    pub generator_version: u8,
    pub protocol_version: u32,
}
