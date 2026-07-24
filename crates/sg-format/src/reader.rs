use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use ed25519_dalek::{Signature, VerifyingKey};

use crate::{
    PackageError, PackageLimits, PackageManifest, Result,
    crypto::{key_id, sha256},
    header::{HEADER_SIZE, Header},
};

pub struct PackageReader {
    pub(crate) path: PathBuf,
    pub(crate) header: Header,
    pub(crate) manifest_bytes: Vec<u8>,
    manifest: PackageManifest,
    limits: PackageLimits,
}

impl PackageReader {
    pub fn open(path: impl AsRef<Path>, limits: PackageLimits) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let header = Header::read(&mut file)?;
        validate_lengths(&file, &header, limits)?;

        let mut manifest_bytes = vec![0_u8; header.manifest_len as usize];
        file.read_exact(&mut manifest_bytes)?;
        if sha256(&manifest_bytes) != header.manifest_hash {
            return Err(PackageError::ManifestHashMismatch);
        }
        let manifest: PackageManifest = serde_json::from_slice(&manifest_bytes)?;
        manifest.validate()?;

        Ok(Self {
            path,
            header,
            manifest_bytes,
            manifest,
            limits,
        })
    }

    pub fn manifest(&self) -> &PackageManifest {
        &self.manifest
    }

    pub fn signer_key_id(&self) -> crate::KeyId {
        self.header.key_id
    }

    pub fn verify_signature(&self, trusted_key: &VerifyingKey) -> Result<()> {
        if key_id(trusted_key) != self.header.key_id {
            return Err(PackageError::UnexpectedSigningKey);
        }
        let signature = Signature::from_bytes(&self.header.signature);
        trusted_key
            .verify_strict(
                &self.header.signed_message(&self.manifest_bytes),
                &signature,
            )
            .map_err(|_| PackageError::InvalidSignature)
    }

    pub fn verify_payload(&self) -> Result<()> {
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.payload_offset()))?;
        let mut payload = file.take(self.header.payload_len);
        let mut hasher = sha2::Sha256::new();
        std::io::copy(&mut payload, &mut DigestWriter(&mut hasher))?;
        use sha2::Digest;
        let hash: [u8; 32] = hasher.finalize().into();
        if hash != self.header.payload_hash {
            return Err(PackageError::PayloadHashMismatch);
        }
        Ok(())
    }

    pub(crate) fn payload_offset(&self) -> u64 {
        HEADER_SIZE as u64 + u64::from(self.header.manifest_len)
    }

    pub(crate) fn limits(&self) -> PackageLimits {
        self.limits
    }
}

fn validate_lengths(file: &File, header: &Header, limits: PackageLimits) -> Result<()> {
    for (field, actual, maximum) in [
        (
            "manifest",
            u64::from(header.manifest_len),
            u64::from(limits.max_manifest_bytes),
        ),
        (
            "compressed payload",
            header.payload_len,
            limits.max_compressed_bytes,
        ),
        (
            "executable",
            header.executable_len,
            limits.max_executable_bytes,
        ),
    ] {
        if actual > maximum {
            return Err(PackageError::LimitExceeded {
                field,
                actual,
                maximum,
            });
        }
    }
    let expected = HEADER_SIZE as u64 + u64::from(header.manifest_len) + header.payload_len;
    if file.metadata()?.len() != expected {
        return Err(PackageError::InvalidFileLength);
    }
    Ok(())
}

struct DigestWriter<'a, D>(&'a mut D);

impl<D: sha2::Digest> std::io::Write for DigestWriter<'_, D> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
