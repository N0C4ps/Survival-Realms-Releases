use super::header::{MAGIC, VERSION};
use crate::application::core::persistence::format::LevelSnapshot;

pub(crate) fn encode(level: &LevelSnapshot) -> Result<Vec<u8>, String> {
    let serialized = bincode::serialize(level).map_err(|error| error.to_string())?;
    let compressed = lz4_flex::compress_prepend_size(&serialized);
    let mut file = Vec::with_capacity(MAGIC.len() + 4 + compressed.len());
    file.extend_from_slice(&MAGIC);
    file.extend_from_slice(&VERSION.to_le_bytes());
    file.extend_from_slice(&compressed);
    Ok(file)
}
