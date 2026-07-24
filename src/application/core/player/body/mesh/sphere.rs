use std::f32::consts::{PI, TAU};

use glam::{Mat4, Vec3};

use super::BodyVertex;

const LATITUDE_SEGMENTS: usize = 8;
const LONGITUDE_SEGMENTS: usize = 12;

pub(super) fn append(
    vertices: &mut Vec<BodyVertex>,
    transform: Mat4,
    normal_rotation: Mat4,
    color: [f32; 3],
) {
    for latitude in 0..LATITUDE_SEGMENTS {
        let top = latitude as f32 / LATITUDE_SEGMENTS as f32;
        let bottom = (latitude + 1) as f32 / LATITUDE_SEGMENTS as f32;
        for longitude in 0..LONGITUDE_SEGMENTS {
            let left = longitude as f32 / LONGITUDE_SEGMENTS as f32;
            let right = (longitude + 1) as f32 / LONGITUDE_SEGMENTS as f32;
            let corners = [
                point(top, left),
                point(bottom, left),
                point(bottom, right),
                point(top, right),
            ];
            for index in [0, 1, 2, 0, 2, 3] {
                let normal = corners[index];
                vertices.push(BodyVertex {
                    position: transform.transform_point3(normal).to_array(),
                    normal: normal_rotation
                        .transform_vector3(normal)
                        .normalize()
                        .to_array(),
                    color,
                });
            }
        }
    }
}

fn point(latitude: f32, longitude: f32) -> Vec3 {
    let vertical = PI * latitude;
    let horizontal = TAU * longitude;
    Vec3::new(
        vertical.sin() * horizontal.cos(),
        vertical.cos(),
        vertical.sin() * horizontal.sin(),
    )
}
