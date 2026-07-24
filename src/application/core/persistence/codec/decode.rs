use std::collections::HashMap;

use serde::Deserialize;

use super::header::{
    HEADER_SIZE, LEGACY_VERSION, MAGIC, MAX_LEVEL_BYTES, PREVIOUS_VERSION, VERSION,
};
use crate::application::core::persistence::format::{LevelSnapshot, SavedBlock, SavedTerrain};

#[derive(Deserialize)]
struct LegacyLevelSnapshot {
    seed: u64,
    terrain: SavedTerrain,
    chunks: HashMap<[i32; 3], Vec<SavedBlock>>,
}

#[derive(Deserialize)]
struct PreviousLevelSnapshot {
    seed: u64,
    terrain: SavedTerrain,
    generator_version: u8,
    chunks: HashMap<[i32; 3], Vec<SavedBlock>>,
}

pub(crate) fn decode(file: &[u8]) -> Result<LevelSnapshot, String> {
    if file.len() < HEADER_SIZE + 4 {
        return Err("level file is truncated".to_owned());
    }
    if file[..MAGIC.len()] != MAGIC {
        return Err("level file has an invalid signature".to_owned());
    }
    let version = u32::from_le_bytes(file[MAGIC.len()..HEADER_SIZE].try_into().unwrap());
    if version != VERSION && version != PREVIOUS_VERSION && version != LEGACY_VERSION {
        return Err(format!(
            "unsupported level version {version}; expected {VERSION}"
        ));
    }

    let compressed = &file[HEADER_SIZE..];
    let declared_size = u32::from_le_bytes(compressed[..4].try_into().unwrap()) as usize;
    if declared_size > MAX_LEVEL_BYTES {
        return Err(format!(
            "level expands to {declared_size} bytes, above the safety limit"
        ));
    }
    let serialized =
        lz4_flex::decompress_size_prepended(compressed).map_err(|error| error.to_string())?;
    match version {
        LEGACY_VERSION => {
            let legacy: LegacyLevelSnapshot =
                bincode::deserialize(&serialized).map_err(|error| error.to_string())?;
            Ok(LevelSnapshot::from_legacy(
                legacy.seed,
                legacy.terrain,
                legacy.chunks,
            ))
        }
        PREVIOUS_VERSION => {
            let previous: PreviousLevelSnapshot =
                bincode::deserialize(&serialized).map_err(|error| error.to_string())?;
            Ok(LevelSnapshot::from_previous(
                previous.seed,
                previous.terrain,
                previous.generator_version,
                previous.chunks,
            ))
        }
        VERSION => bincode::deserialize(&serialized).map_err(|error| error.to_string()),
        _ => unreachable!("level version was validated above"),
    }
}
