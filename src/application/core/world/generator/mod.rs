mod controller;
mod kind;
mod legacy_flat;
mod procedural;
mod settings;

pub use controller::WorldGenerator;
pub use kind::GeneratorKind;
pub use settings::TerrainDimensions;

pub(crate) const GENERATOR_VERSION: u8 = 9;
