use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use ed25519_dalek::{SigningKey, VerifyingKey};

const PRIVATE_MAGIC: [u8; 8] = *b"SGPRIV\0\0";
const PUBLIC_MAGIC: [u8; 8] = *b"SGPUB\0\0\0";
const KEY_FORMAT_VERSION: u16 = 1;
const KEY_FILE_SIZE: u64 = 8 + 2 + 32;

pub(crate) fn write_pair(path: &Path, signing_key: &SigningKey) -> Result<PathBuf, String> {
    let public_path = public_path(path);
    if path.exists() || public_path.exists() {
        return Err("refusing to overwrite an existing signing key".to_owned());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let result = (|| {
        write_private(path, signing_key)?;
        write_key_file(
            &public_path,
            &PUBLIC_MAGIC,
            signing_key.verifying_key().as_bytes(),
        )?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(&public_path);
    }
    result.map(|()| public_path)
}

pub(crate) fn read_private(path: &Path) -> Result<SigningKey, String> {
    let mut bytes = read_key_file(path, &PRIVATE_MAGIC)?;
    let key = SigningKey::from_bytes(&bytes);
    bytes.fill(0);
    Ok(key)
}

#[allow(dead_code)]
pub(crate) fn read_public(path: &Path) -> Result<VerifyingKey, String> {
    let bytes = read_key_file(path, &PUBLIC_MAGIC)?;
    VerifyingKey::from_bytes(&bytes).map_err(|error| error.to_string())
}

fn write_private(path: &Path, signing_key: &SigningKey) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|error| error.to_string())?;
        write_contents(&mut file, &PRIVATE_MAGIC, &signing_key.to_bytes())
    }
    #[cfg(not(unix))]
    {
        write_key_file(path, &PRIVATE_MAGIC, &signing_key.to_bytes())
    }
}

fn write_key_file(path: &Path, magic: &[u8; 8], key: &[u8; 32]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    write_contents(&mut file, magic, key)
}

fn write_contents(file: &mut File, magic: &[u8; 8], key: &[u8; 32]) -> Result<(), String> {
    file.write_all(magic).map_err(|error| error.to_string())?;
    file.write_all(&KEY_FORMAT_VERSION.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(key).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn read_key_file(path: &Path, expected_magic: &[u8; 8]) -> Result<[u8; 32], String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() != KEY_FILE_SIZE {
        return Err(format!(
            "invalid signing-key file length: {}",
            path.display()
        ));
    }
    let mut header = [0_u8; 10];
    file.read_exact(&mut header)
        .map_err(|error| error.to_string())?;
    if &header[..8] != expected_magic {
        return Err(format!("invalid signing-key file: {}", path.display()));
    }
    let version = u16::from_le_bytes(header[8..10].try_into().unwrap());
    if version != KEY_FORMAT_VERSION {
        return Err(format!("unsupported signing-key format {version}"));
    }
    let mut key = [0_u8; 32];
    file.read_exact(&mut key)
        .map_err(|error| error.to_string())?;
    Ok(key)
}

fn public_path(private: &Path) -> PathBuf {
    let name = private.file_name().unwrap_or_default().to_string_lossy();
    private.with_file_name(format!("{name}.pub"))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn key_pair_round_trips_without_overwriting() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("sg-key-{unique}"));
        let private = directory.join("release.sgkey");
        let key = SigningKey::from_bytes(&[42; 32]);

        let public = write_pair(&private, &key).unwrap();
        assert_eq!(read_private(&private).unwrap().to_bytes(), key.to_bytes());
        assert_eq!(read_public(&public).unwrap(), key.verifying_key());
        assert!(write_pair(&private, &key).is_err());

        fs::remove_dir_all(directory).unwrap();
    }
}
