use glam::{IVec3, Vec3};

#[derive(Clone, Copy, Debug)]
pub(crate) struct DebugSnapshot {
    pub fps: f64,
    pub player_position: Vec3,
    pub player_chunk: IVec3,
    pub render_distance: u32,
    pub loaded_chunks: usize,
}
