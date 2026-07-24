use glam::{Mat4, Vec3};

use super::BodyVertex;

const FACES: [(Vec3, [Vec3; 4]); 6] = [
    (
        Vec3::X,
        [
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(0.5, -0.5, 0.5),
        ],
    ),
    (
        Vec3::NEG_X,
        [
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(-0.5, 0.5, 0.5),
            Vec3::new(-0.5, 0.5, -0.5),
            Vec3::new(-0.5, -0.5, -0.5),
        ],
    ),
    (
        Vec3::Y,
        [
            Vec3::new(-0.5, 0.5, -0.5),
            Vec3::new(-0.5, 0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(0.5, 0.5, -0.5),
        ],
    ),
    (
        Vec3::NEG_Y,
        [
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, 0.5),
        ],
    ),
    (
        Vec3::Z,
        [
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(-0.5, 0.5, 0.5),
            Vec3::new(-0.5, -0.5, 0.5),
        ],
    ),
    (
        Vec3::NEG_Z,
        [
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(-0.5, 0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
        ],
    ),
];

pub(super) fn append(
    vertices: &mut Vec<BodyVertex>,
    transform: Mat4,
    normal_rotation: Mat4,
    color: [f32; 3],
) {
    for (normal, corners) in FACES {
        for index in [0, 1, 2, 0, 2, 3] {
            let position = transform.transform_point3(corners[index]);
            let normal = normal_rotation.transform_vector3(normal).normalize();
            vertices.push(BodyVertex {
                position: position.to_array(),
                normal: normal.to_array(),
                color,
            });
        }
    }
}
