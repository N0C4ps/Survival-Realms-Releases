use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::VerifyingKey;
use sha2::{Digest, Sha256};

use crate::{PackageError, PackageReader, Result};

impl PackageReader {
    pub fn extract_executable(
        &self,
        destination_dir: impl AsRef<Path>,
        trusted_key: &VerifyingKey,
    ) -> Result<PathBuf> {
        self.verify_signature(trusted_key)?;
        self.verify_payload()?;
        fs::create_dir_all(destination_dir.as_ref())?;
        let destination = destination_dir.as_ref().join(&self.manifest().executable);
        if destination.exists() {
            if hash_file(&destination)? == self.header.executable_hash {
                return Ok(destination);
            }
            return Err(PackageError::DestinationConflict(destination));
        }

        let temporary = temporary_path(&destination);
        let result = self.extract_to_temporary(&temporary);
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        fs::rename(&temporary, &destination)?;
        Ok(destination)
    }

    fn extract_to_temporary(&self, temporary: &Path) -> Result<()> {
        let output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temporary)?;
        let mut package = File::open(&self.path)?;
        package.seek(SeekFrom::Start(self.payload_offset()))?;
        let payload = package.take(self.header.payload_len);
        let mut decoder = zstd::stream::read::Decoder::new(payload)?;
        let mut output = HashingWriter::new(output);
        let mut buffer = [0_u8; 64 * 1024];
        let mut written = 0_u64;

        loop {
            let read = decoder.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            written = written
                .checked_add(read as u64)
                .ok_or(PackageError::LimitExceeded {
                    field: "executable",
                    actual: u64::MAX,
                    maximum: self.limits().max_executable_bytes,
                })?;
            if written > self.header.executable_len || written > self.limits().max_executable_bytes
            {
                return Err(PackageError::LimitExceeded {
                    field: "executable",
                    actual: written,
                    maximum: self.header.executable_len,
                });
            }
            output.write_all(&buffer[..read])?;
        }
        if written != self.header.executable_len {
            return Err(PackageError::ExecutableHashMismatch);
        }
        let (mut file, hash) = output.finish();
        if hash != self.header.executable_hash {
            return Err(PackageError::ExecutableHashMismatch);
        }
        file.flush()?;
        file.sync_all()?;
        Ok(())
    }
}

fn hash_file(path: &Path) -> Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn temporary_path(destination: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    destination.with_file_name(format!(".{name}.sg-part-{}-{nonce}", std::process::id()))
}

struct HashingWriter<W> {
    writer: W,
    hasher: Sha256,
}

impl<W> HashingWriter<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            hasher: Sha256::new(),
        }
    }

    fn finish(self) -> (W, [u8; 32]) {
        (self.writer, self.hasher.finalize().into())
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = self.writer.write(bytes)?;
        self.hasher.update(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
