use bytemuck::{Pod, Zeroable};
use glam::IVec3;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct HighlightUniform {
    block_origin: [f32; 4],
}

impl From<IVec3> for HighlightUniform {
    fn from(block: IVec3) -> Self {
        Self {
            block_origin: [block.x as f32, block.y as f32, block.z as f32, 0.0],
        }
    }
}
