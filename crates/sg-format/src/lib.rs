mod builder;
mod crypto;
mod error;
mod extract;
mod header;
mod limits;
mod manifest;
mod reader;
mod remote_index;

pub use builder::PackageBuilder;
pub use crypto::{KeyId, key_id};
pub use error::{PackageError, Result};
pub use limits::PackageLimits;
pub use manifest::{PackageManifest, ReleaseChannel};
pub use reader::PackageReader;
pub use remote_index::{
    INDEX_SCHEMA_VERSION, RemoteAssetFile, RemoteAssetPack, RemoteVersion, SignedVersionIndex,
    VersionIndex,
};

#[cfg(test)]
mod tests;
