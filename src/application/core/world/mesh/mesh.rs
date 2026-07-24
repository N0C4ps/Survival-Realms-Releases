use glam::IVec3;

use super::Vertex;

pub struct ChunkMesh {
    pub chunk_position: IVec3,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub liquid_vertices: Vec<Vertex>,
    pub liquid_indices: Vec<u32>,
}

impl ChunkMesh {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() && self.liquid_indices.is_empty()
    }

    pub fn has_opaque_geometry(&self) -> bool {
        !self.indices.is_empty()
    }

    pub fn has_liquid_geometry(&self) -> bool {
        !self.liquid_indices.is_empty()
    }
}
