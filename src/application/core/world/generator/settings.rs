#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerrainDimensions {
    chunks_x: u32,
    chunks_z: u32,
    chunks_below_sea_level: u32,
    chunks_above_sea_level: u32,
}

impl TerrainDimensions {
    pub const fn new(
        chunks_x: u32,
        chunks_z: u32,
        chunks_below_sea_level: u32,
        chunks_above_sea_level: u32,
    ) -> Self {
        assert!(chunks_x > 0, "terrain must contain chunks on the X axis");
        assert!(chunks_z > 0, "terrain must contain chunks on the Z axis");
        assert!(
            chunks_above_sea_level > 0,
            "terrain must contain the chunk directly above sea level"
        );

        Self {
            chunks_x,
            chunks_z,
            chunks_below_sea_level,
            chunks_above_sea_level,
        }
    }

    pub const fn chunks_x(self) -> u32 {
        self.chunks_x
    }

    pub const fn chunks_z(self) -> u32 {
        self.chunks_z
    }

    pub const fn chunks_below_sea_level(self) -> u32 {
        self.chunks_below_sea_level
    }

    pub const fn chunks_above_sea_level(self) -> u32 {
        self.chunks_above_sea_level
    }

    pub const fn total_chunks(self) -> u32 {
        self.chunks_x * self.chunks_z * (self.chunks_below_sea_level + self.chunks_above_sea_level)
    }
}

impl Default for TerrainDimensions {
    fn default() -> Self {
        Self::new(512, 512, 32, 32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dimensions_support_a_large_streamed_world() {
        let dimensions = TerrainDimensions::default();

        assert_eq!(dimensions.chunks_x(), 512);
        assert_eq!(dimensions.chunks_z(), 512);
        assert_eq!(dimensions.chunks_below_sea_level(), 32);
        assert_eq!(dimensions.chunks_above_sea_level(), 32);
        assert_eq!(dimensions.total_chunks(), 16_777_216);
    }
}
