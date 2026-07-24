use std::{sync::mpsc::Receiver, thread};

use super::{super::storage::LevelStorage, command::SaveCommand};

pub(super) fn spawn(
    storage: LevelStorage,
    receiver: Receiver<SaveCommand>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("level-save-worker".to_owned())
        .spawn(move || run(storage, receiver))
        .expect("failed to start the level save worker")
}

fn run(storage: LevelStorage, receiver: Receiver<SaveCommand>) {
    let mut last_result = Ok(());
    while let Ok(command) = receiver.recv() {
        match command {
            SaveCommand::Save(level) => {
                last_result = storage.save(&level);
                match &last_result {
                    Ok(()) => tracing::info!(path = %storage.path().display(), "level saved"),
                    Err(error) => tracing::error!(%error, "failed to save level"),
                }
            }
            SaveCommand::Flush(reply) => {
                let _ = reply.send(last_result.clone());
            }
            SaveCommand::Shutdown => break,
        }
    }
}
