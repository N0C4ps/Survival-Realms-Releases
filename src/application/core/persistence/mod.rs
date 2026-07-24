mod codec;
mod compatibility;
mod format;
mod service;
mod storage;

pub(crate) use codec::SAVE_FORMAT_VERSION;
pub(crate) use compatibility::{inspect_save, prepare_save_for_launch};
pub(crate) use format::LevelSnapshot;
pub(crate) use service::PersistenceService;

pub(crate) const MIN_SUPPORTED_SAVE_FORMAT: u32 = 1;
