use glam::{IVec3, Mat4, Vec4};

use crate::application::core::world::CHUNK_SIZE;

pub(super) struct Frustum {
    view_projection: Mat4,
}

impl Frustum {
    pub fn new(view_projection: Mat4) -> Self {
        Self { view_projection }
    }

    pub fn contains_chunk(&self, chunk: IVec3) -> bool {
        let minimum = (chunk * CHUNK_SIZE as i32).as_vec3();
        let maximum = minimum + glam::Vec3::splat(CHUNK_SIZE as f32);
        let corners = [
            [minimum.x, minimum.y, minimum.z],
            [maximum.x, minimum.y, minimum.z],
            [minimum.x, maximum.y, minimum.z],
            [maximum.x, maximum.y, minimum.z],
            [minimum.x, minimum.y, maximum.z],
            [maximum.x, minimum.y, maximum.z],
            [minimum.x, maximum.y, maximum.z],
            [maximum.x, maximum.y, maximum.z],
        ]
        .map(|point| self.view_projection * Vec4::new(point[0], point[1], point[2], 1.0));

        !outside_every_corner(&corners, |point| point.x < -point.w)
            && !outside_every_corner(&corners, |point| point.x > point.w)
            && !outside_every_corner(&corners, |point| point.y < -point.w)
            && !outside_every_corner(&corners, |point| point.y > point.w)
            && !outside_every_corner(&corners, |point| point.z < 0.0)
            && !outside_every_corner(&corners, |point| point.z > point.w)
    }
}

fn outside_every_corner(corners: &[Vec4; 8], outside: impl Fn(&Vec4) -> bool) -> bool {
    corners.iter().all(outside)
}
