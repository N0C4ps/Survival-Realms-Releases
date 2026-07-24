use ed25519_dalek::VerifyingKey;
use sha2::{Digest, Sha256};

pub type KeyId = [u8; 16];

pub fn key_id(key: &VerifyingKey) -> KeyId {
    let digest = Sha256::digest(key.as_bytes());
    digest[..16]
        .try_into()
        .expect("SHA-256 prefix has 16 bytes")
}

pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
