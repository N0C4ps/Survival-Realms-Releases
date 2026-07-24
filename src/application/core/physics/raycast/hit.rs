use glam::{IVec3, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaycastHit {
    pub block: IVec3,
    pub normal: IVec3,
    pub distance: f32,
    pub point: Vec3,
}
