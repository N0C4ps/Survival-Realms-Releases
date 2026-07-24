use glam::{IVec3, Vec3};

const BLOCK_RANGE_EPSILON: f32 = 0.000_1;

pub(super) struct Aabb {
    pub minimum: Vec3,
    pub maximum: Vec3,
}

impl Aabb {
    pub fn intersects_with_tolerance(&self, other: &Self, tolerance: f32) -> bool {
        self.minimum.x < other.maximum.x - tolerance
            && self.maximum.x > other.minimum.x + tolerance
            && self.minimum.y < other.maximum.y - tolerance
            && self.maximum.y > other.minimum.y + tolerance
            && self.minimum.z < other.maximum.z - tolerance
            && self.maximum.z > other.minimum.z + tolerance
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            minimum: self.minimum.min(other.minimum),
            maximum: self.maximum.max(other.maximum),
        }
    }

    pub fn block_minimum(&self) -> IVec3 {
        (self.minimum + Vec3::splat(BLOCK_RANGE_EPSILON))
            .floor()
            .as_ivec3()
    }

    pub fn block_maximum(&self) -> IVec3 {
        (self.maximum - Vec3::splat(BLOCK_RANGE_EPSILON))
            .floor()
            .as_ivec3()
    }
}
