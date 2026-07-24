use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) struct SavedBlock {
    pub local_index: u16,
    pub block_id: u8,
}
