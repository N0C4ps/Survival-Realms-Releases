mod command;
mod worker;

use std::{
    sync::mpsc::{self, Sender},
    thread::JoinHandle,
};

use super::{format::LevelSnapshot, storage::LevelStorage};
use command::SaveCommand;

pub(crate) struct PersistenceService {
    storage: LevelStorage,
    sender: Sender<SaveCommand>,
    worker: Option<JoinHandle<()>>,
}

impl PersistenceService {
    pub fn new(level_path: std::path::PathBuf) -> Self {
        let storage = LevelStorage::new(level_path);
        let (sender, receiver) = mpsc::channel();
        let worker = worker::spawn(storage.clone(), receiver);
        Self {
            storage,
            sender,
            worker: Some(worker),
        }
    }

    pub fn level_path(&self) -> &std::path::Path {
        self.storage.path()
    }

    pub fn load(&self) -> Result<Option<LevelSnapshot>, String> {
        self.storage.load()
    }

    pub fn quarantine_corrupt_level(&self) -> Result<Option<std::path::PathBuf>, String> {
        self.storage.quarantine_corrupt_level()
    }

    pub fn save_async(&self, level: LevelSnapshot) -> Result<(), String> {
        self.sender
            .send(SaveCommand::Save(level))
            .map_err(|error| error.to_string())
    }

    pub fn flush(&self) -> Result<(), String> {
        let (sender, receiver) = mpsc::channel();
        self.sender
            .send(SaveCommand::Flush(sender))
            .map_err(|error| error.to_string())?;
        receiver.recv().map_err(|error| error.to_string())?
    }
}

impl Drop for PersistenceService {
    fn drop(&mut self) {
        let _ = self.sender.send(SaveCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
