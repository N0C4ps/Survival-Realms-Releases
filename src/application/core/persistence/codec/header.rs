pub(crate) const MAGIC: [u8; 8] = *b"SGLEVEL\0";
pub(super) const LEGACY_VERSION: u32 = 1;
pub(super) const PREVIOUS_VERSION: u32 = 2;
pub(super) const VERSION: u32 = 3;
pub(crate) const HEADER_SIZE: usize = MAGIC.len() + std::mem::size_of::<u32>();
pub(super) const MAX_LEVEL_BYTES: usize = 64 * 1024 * 1024;
