use std::io::{Read, Write};

use crate::{KeyId, PackageError, Result};

pub(crate) const MAGIC: [u8; 8] = *b"SGVER\0\0\0";
pub(crate) const FORMAT_VERSION: u16 = 1;
pub(crate) const FLAG_SIGNED: u16 = 1;
pub(crate) const SUPPORTED_FLAGS: u16 = FLAG_SIGNED;
pub(crate) const SIGNATURE_SIZE: usize = 64;
pub(crate) const HEADER_SIZE: usize = 208;
const SIGNED_PREFIX_SIZE: usize = HEADER_SIZE - SIGNATURE_SIZE;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Header {
    pub flags: u16,
    pub manifest_len: u32,
    pub payload_len: u64,
    pub executable_len: u64,
    pub manifest_hash: [u8; 32],
    pub payload_hash: [u8; 32],
    pub executable_hash: [u8; 32],
    pub key_id: KeyId,
    pub signature: [u8; SIGNATURE_SIZE],
}

impl Header {
    pub fn read(mut reader: impl Read) -> Result<Self> {
        let mut bytes = [0_u8; HEADER_SIZE];
        reader.read_exact(&mut bytes)?;
        if bytes[..8] != MAGIC {
            return Err(PackageError::InvalidMagic);
        }
        let format = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
        if format != FORMAT_VERSION {
            return Err(PackageError::UnsupportedFormat(format));
        }
        let flags = u16::from_le_bytes(bytes[10..12].try_into().unwrap());
        if flags & !SUPPORTED_FLAGS != 0 || flags & FLAG_SIGNED == 0 {
            return Err(PackageError::UnsupportedFlags(flags));
        }
        Ok(Self {
            flags,
            manifest_len: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            payload_len: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            executable_len: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            manifest_hash: bytes[32..64].try_into().unwrap(),
            payload_hash: bytes[64..96].try_into().unwrap(),
            executable_hash: bytes[96..128].try_into().unwrap(),
            key_id: bytes[128..144].try_into().unwrap(),
            signature: bytes[144..208].try_into().unwrap(),
        })
    }

    pub fn write(&self, mut writer: impl Write) -> Result<()> {
        writer.write_all(&self.bytes())?;
        Ok(())
    }

    pub fn signed_message(&self, manifest: &[u8]) -> Vec<u8> {
        let bytes = self.bytes();
        let mut message = Vec::with_capacity(SIGNED_PREFIX_SIZE + manifest.len());
        message.extend_from_slice(&bytes[..SIGNED_PREFIX_SIZE]);
        message.extend_from_slice(manifest);
        message
    }

    fn bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0_u8; HEADER_SIZE];
        bytes[..8].copy_from_slice(&MAGIC);
        bytes[8..10].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.flags.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.manifest_len.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.executable_len.to_le_bytes());
        bytes[32..64].copy_from_slice(&self.manifest_hash);
        bytes[64..96].copy_from_slice(&self.payload_hash);
        bytes[96..128].copy_from_slice(&self.executable_hash);
        bytes[128..144].copy_from_slice(&self.key_id);
        bytes[144..208].copy_from_slice(&self.signature);
        bytes
    }
}
