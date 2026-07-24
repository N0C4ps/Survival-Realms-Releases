use std::sync::mpsc::Sender;

use super::super::format::LevelSnapshot;

pub(super) enum SaveCommand {
    Save(LevelSnapshot),
    Flush(Sender<Result<(), String>>),
    Shutdown,
}
