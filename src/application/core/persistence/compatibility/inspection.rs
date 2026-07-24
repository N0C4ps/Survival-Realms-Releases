use std::{
    fs::File,
    io::{ErrorKind, Read},
    path::Path,
};

use super::{
    super::codec::{HEADER_SIZE, MAGIC},
    SaveInspection,
};

const MINIMUM_FILE_SIZE: u64 = (HEADER_SIZE + size_of::<u32>()) as u64;

pub(crate) fn inspect_save(path: &Path) -> Result<SaveInspection, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(SaveInspection::missing(path));
        }
        Err(error) => return Err(format!("failed to inspect {}: {error}", path.display())),
    };
    let length = file
        .metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?
        .len();
    if length < MINIMUM_FILE_SIZE {
        return Ok(SaveInspection::corrupt(path, "level file is truncated"));
    }

    let mut header = [0_u8; HEADER_SIZE];
    file.read_exact(&mut header)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if header[..MAGIC.len()] != MAGIC {
        return Ok(SaveInspection::corrupt(
            path,
            "level file has an invalid signature",
        ));
    }
    let version = u32::from_le_bytes(header[MAGIC.len()..].try_into().unwrap());
    Ok(SaveInspection::from_version(path, version))
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use super::*;
    use crate::application::core::persistence::SAVE_FORMAT_VERSION;
    use crate::application::core::persistence::compatibility::SaveStatus;

    fn test_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("sg-{name}-{}-{unique}.level", std::process::id()))
    }

    fn write_header(path: &Path, magic: &[u8; 8], version: u32) {
        let mut bytes = Vec::from(*magic);
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn inspection_classifies_save_versions_without_decoding_payload() {
        for (version, expected) in [
            (1, SaveStatus::MigrationRequired),
            (SAVE_FORMAT_VERSION, SaveStatus::Ready),
            (SAVE_FORMAT_VERSION + 1, SaveStatus::NewerThanGame),
        ] {
            let path = test_path("inspection");
            write_header(&path, &MAGIC, version);
            let inspection = inspect_save(&path).unwrap();
            fs::remove_file(path).unwrap();

            assert_eq!(inspection.status(), expected);
            assert_eq!(inspection.format_version(), Some(version));
        }
    }

    #[test]
    fn invalid_signature_is_reported_as_corrupt() {
        let path = test_path("invalid-signature");
        write_header(&path, b"NOTLEVEL", SAVE_FORMAT_VERSION);
        let inspection = inspect_save(&path).unwrap();
        fs::remove_file(path).unwrap();

        assert_eq!(inspection.status(), SaveStatus::Corrupt);
    }
}
