use serde::{Deserialize, Serialize};

use crate::application::core::world::TerrainDimensions;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) struct SavedTerrain {
    chunks_x: u32,
    chunks_z: u32,
    chunks_below_sea_level: u32,
    chunks_above_sea_level: u32,
}

impl From<TerrainDimensions> for SavedTerrain {
    fn from(dimensions: TerrainDimensions) -> Self {
        Self {
            chunks_x: dimensions.chunks_x(),
            chunks_z: dimensions.chunks_z(),
            chunks_below_sea_level: dimensions.chunks_below_sea_level(),
            chunks_above_sea_level: dimensions.chunks_above_sea_level(),
        }
    }
}

impl SavedTerrain {
    pub fn dimensions(self) -> Result<TerrainDimensions, String> {
        if self.chunks_x == 0 || self.chunks_z == 0 || self.chunks_above_sea_level == 0 {
            return Err("level contains invalid terrain dimensions".to_owned());
        }
        if self.chunks_x > 4_096
            || self.chunks_z > 4_096
            || self.chunks_below_sea_level > 256
            || self.chunks_above_sea_level > 256
        {
            return Err("level terrain dimensions exceed safety limits".to_owned());
        }
        Ok(TerrainDimensions::new(
            self.chunks_x,
            self.chunks_z,
            self.chunks_below_sea_level,
            self.chunks_above_sea_level,
        ))
    }
}
