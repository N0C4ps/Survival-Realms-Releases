use glam::{BVec3, Vec3};

pub(crate) struct CollisionResult {
    pub position: Vec3,
    pub blocked_axes: BVec3,
    pub grounded: bool,
}
