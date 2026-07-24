use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{KeyId, PackageError, PackageManifest, Result, key_id};

pub const INDEX_SCHEMA_VERSION: u16 = 2;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteAssetFile {
    pub path: String,
    pub file_url: String,
    pub file_size: u64,
    pub file_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteAssetPack {
    pub id: String,
    pub files: Vec<RemoteAssetFile>,
}

impl RemoteAssetPack {
    pub fn total_size(&self) -> Option<u64> {
        self.files
            .iter()
            .try_fold(0_u64, |total, file| total.checked_add(file.file_size))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteVersion {
    pub manifest: PackageManifest,
    pub package_url: String,
    pub package_size: u64,
    pub package_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VersionIndex {
    pub schema_version: u16,
    pub generated_at: u64,
    pub versions: Vec<RemoteVersion>,
    pub asset_packs: Vec<RemoteAssetPack>,
}

impl VersionIndex {
    pub fn find(&self, version: &Version) -> Option<&RemoteVersion> {
        self.versions
            .iter()
            .find(|entry| &entry.manifest.version == version)
    }

    pub fn find_asset_pack(&self, id: &str) -> Option<&RemoteAssetPack> {
        self.asset_packs.iter().find(|pack| pack.id == id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedVersionIndex {
    pub index: VersionIndex,
    pub signer_key_id: String,
    pub signature: String,
}

impl SignedVersionIndex {
    pub fn sign(index: VersionIndex, key: &SigningKey) -> Result<Self> {
        let message = serde_json::to_vec(&index)?;
        Ok(Self {
            index,
            signer_key_id: hex::encode(key_id(&key.verifying_key())),
            signature: hex::encode(key.sign(&message).to_bytes()),
        })
    }

    pub fn signer_key_id(&self) -> Result<KeyId> {
        decode_fixed(&self.signer_key_id, "key id")
    }

    pub fn verify(&self, key: &VerifyingKey) -> Result<()> {
        if self.signer_key_id()? != key_id(key) {
            return Err(PackageError::UnexpectedSigningKey);
        }
        let signature = decode_fixed::<64>(&self.signature, "index signature")?;
        let message = serde_json::to_vec(&self.index)?;
        key.verify_strict(&message, &Signature::from_bytes(&signature))
            .map_err(|_| PackageError::InvalidSignature)
    }
}

fn decode_fixed<const N: usize>(encoded: &str, name: &str) -> Result<[u8; N]> {
    let bytes = hex::decode(encoded)
        .map_err(|error| PackageError::InvalidIndex(format!("invalid {name}: {error}")))?;
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        PackageError::InvalidIndex(format!("{name} has {} bytes, expected {N}", bytes.len()))
    })
}
