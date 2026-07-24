#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageLimits {
    pub max_manifest_bytes: u32,
    pub max_compressed_bytes: u64,
    pub max_executable_bytes: u64,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_manifest_bytes: 64 * 1024,
            max_compressed_bytes: 256 * 1024 * 1024,
            max_executable_bytes: 512 * 1024 * 1024,
        }
    }
}
