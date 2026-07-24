use glam::{Mat4, Quat, Vec3};

use super::{BodyVertex, cuboid, sphere};
use crate::application::core::player::body::{
    BodyPose,
    dimensions::{
        ARM_CENTER_Y, ARM_SIZE, ARM_X, HEAD_CENTER_Y, HEAD_RADIUS, LEG_CENTER_Y, LEG_SIZE, LEG_X,
        TORSO_CENTER_Y, TORSO_SIZE,
    },
};

const SKIN_COLOR: [f32; 3] = [0.95, 0.72, 0.30];
const TORSO_COLOR: [f32; 3] = [0.10, 0.43, 0.68];
const LEG_COLOR: [f32; 3] = [0.10, 0.17, 0.28];

pub(crate) fn append(pose: BodyPose, include_head: bool, vertices: &mut Vec<BodyVertex>) {
    let rotation = Quat::from_rotation_y(pose.yaw());
    let root = Mat4::from_rotation_translation(rotation, pose.position);
    let normal_rotation = Mat4::from_quat(rotation);

    append_cuboid(
        vertices,
        root,
        normal_rotation,
        Vec3::new(0.0, TORSO_CENTER_Y, 0.0),
        Vec3::from_array(TORSO_SIZE),
        TORSO_COLOR,
    );
    for x in [-LEG_X, LEG_X] {
        append_cuboid(
            vertices,
            root,
            normal_rotation,
            Vec3::new(x, LEG_CENTER_Y, 0.0),
            Vec3::from_array(LEG_SIZE),
            LEG_COLOR,
        );
    }
    for x in [-ARM_X, ARM_X] {
        append_cuboid(
            vertices,
            root,
            normal_rotation,
            Vec3::new(x, ARM_CENTER_Y, 0.0),
            Vec3::from_array(ARM_SIZE),
            SKIN_COLOR,
        );
    }
    if include_head {
        sphere::append(
            vertices,
            root * Mat4::from_scale_rotation_translation(
                Vec3::splat(HEAD_RADIUS),
                Quat::IDENTITY,
                Vec3::new(0.0, HEAD_CENTER_Y, 0.0),
            ),
            normal_rotation,
            SKIN_COLOR,
        );
    }
}

#[cfg(test)]
fn build(pose: BodyPose, include_head: bool, vertices: &mut Vec<BodyVertex>) {
    vertices.clear();
    append(pose, include_head, vertices);
}

fn append_cuboid(
    vertices: &mut Vec<BodyVertex>,
    root: Mat4,
    normal_rotation: Mat4,
    center: Vec3,
    size: Vec3,
    color: [f32; 3],
) {
    cuboid::append(
        vertices,
        root * Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, center),
        normal_rotation,
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::core::player::body::dimensions::{BODY_HEIGHT, BODY_MAXIMUM_WIDTH};

    #[test]
    fn complete_character_stays_inside_declared_dimensions() {
        let mut vertices = Vec::new();
        build(BodyPose::new(Vec3::ZERO, Vec3::NEG_Z), true, &mut vertices);
        let minimum = vertices
            .iter()
            .map(|vertex| Vec3::from_array(vertex.position))
            .reduce(Vec3::min)
            .unwrap();
        let maximum = vertices
            .iter()
            .map(|vertex| Vec3::from_array(vertex.position))
            .reduce(Vec3::max)
            .unwrap();

        assert!(maximum.x - minimum.x <= BODY_MAXIMUM_WIDTH);
        assert!(maximum.z - minimum.z < 1.0);
        assert!(minimum.y >= 0.0);
        assert!(maximum.y <= BODY_HEIGHT);
    }

    #[test]
    fn local_first_person_mesh_omits_only_the_spherical_head() {
        let pose = BodyPose::new(Vec3::ZERO, Vec3::NEG_Z);
        let mut local = Vec::new();
        let mut complete = Vec::new();
        build(pose, false, &mut local);
        build(pose, true, &mut complete);

        assert_eq!(local.len(), 5 * 36);
        assert!(complete.len() > local.len());
    }
}
