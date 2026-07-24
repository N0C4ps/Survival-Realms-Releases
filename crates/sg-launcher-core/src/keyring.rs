use std::collections::HashMap;

use ed25519_dalek::VerifyingKey;
use sg_format::{KeyId, key_id};

use crate::{LauncherError, Result};

#[derive(Clone, Default)]
pub struct TrustedKeyring {
    keys: HashMap<KeyId, VerifyingKey>,
}

impl TrustedKeyring {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trust(&mut self, key: VerifyingKey) -> KeyId {
        let id = key_id(&key);
        self.keys.insert(id, key);
        id
    }

    pub fn trust_bytes(&mut self, bytes: &[u8; 32]) -> Result<KeyId> {
        let key = VerifyingKey::from_bytes(bytes)
            .map_err(|error| LauncherError::InvalidPublicKey(error.to_string()))?;
        Ok(self.trust(key))
    }

    pub fn verifying_key(&self, id: &KeyId) -> Option<&VerifyingKey> {
        self.keys.get(id)
    }

    pub(crate) fn get(&self, id: &KeyId) -> Option<&VerifyingKey> {
        self.verifying_key(id)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}
