use glam::{IVec3, Vec3};

use crate::application::core::{blocks::BlockRegistry, world::World};

use super::RaycastHit;

pub fn cast_voxels(
    world: &World,
    registry: &BlockRegistry,
    origin: Vec3,
    direction: Vec3,
    maximum_distance: f32,
) -> Option<RaycastHit> {
    let direction = direction.try_normalize()?;
    let mut block = origin.floor().as_ivec3();

    if is_solid(world, registry, block) {
        return Some(RaycastHit {
            block,
            normal: IVec3::ZERO,
            distance: 0.0,
            point: origin,
        });
    }

    let step = IVec3::new(
        axis_step(direction.x),
        axis_step(direction.y),
        axis_step(direction.z),
    );
    let delta = Vec3::new(
        axis_delta(direction.x),
        axis_delta(direction.y),
        axis_delta(direction.z),
    );
    let mut boundary = Vec3::new(
        first_boundary(origin.x, block.x, direction.x),
        first_boundary(origin.y, block.y, direction.y),
        first_boundary(origin.z, block.z, direction.z),
    );

    loop {
        let (distance, normal) = if boundary.x <= boundary.y && boundary.x <= boundary.z {
            let distance = boundary.x;
            boundary.x += delta.x;
            block.x += step.x;
            (distance, IVec3::new(-step.x, 0, 0))
        } else if boundary.y <= boundary.z {
            let distance = boundary.y;
            boundary.y += delta.y;
            block.y += step.y;
            (distance, IVec3::new(0, -step.y, 0))
        } else {
            let distance = boundary.z;
            boundary.z += delta.z;
            block.z += step.z;
            (distance, IVec3::new(0, 0, -step.z))
        };

        if distance > maximum_distance {
            return None;
        }
        if is_solid(world, registry, block) {
            return Some(RaycastHit {
                block,
                normal,
                distance,
                point: origin + direction * distance,
            });
        }
    }
}

fn is_solid(world: &World, registry: &BlockRegistry, block: IVec3) -> bool {
    registry.get(world.block(block)).properties().is_solid()
}

fn axis_step(direction: f32) -> i32 {
    if direction > 0.0 {
        1
    } else if direction < 0.0 {
        -1
    } else {
        0
    }
}

fn axis_delta(direction: f32) -> f32 {
    if direction == 0.0 {
        f32::INFINITY
    } else {
        direction.recip().abs()
    }
}

fn first_boundary(origin: f32, block: i32, direction: f32) -> f32 {
    if direction > 0.0 {
        (block as f32 + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (origin - block as f32) / -direction
    } else {
        f32::INFINITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::core::blocks::BlockId;

    #[test]
    fn hits_first_solid_voxel_and_reports_the_entered_face() {
        let mut world = World::default();
        world.set_block(IVec3::new(0, 2, -3), BlockId::STONE);

        let hit = cast_voxels(
            &world,
            &BlockRegistry::builtin(),
            Vec3::new(0.5, 2.5, 0.5),
            Vec3::NEG_Z,
            5.0,
        )
        .expect("ray should hit stone");

        assert_eq!(hit.block, IVec3::new(0, 2, -3));
        assert_eq!(hit.normal, IVec3::Z);
        assert!((hit.distance - 2.5).abs() < 0.000_1);
    }

    #[test]
    fn ignores_solids_beyond_the_maximum_reach() {
        let mut world = World::default();
        world.set_block(IVec3::new(0, 2, -6), BlockId::STONE);

        assert!(
            cast_voxels(
                &world,
                &BlockRegistry::builtin(),
                Vec3::new(0.5, 2.5, 0.5),
                Vec3::NEG_Z,
                5.0,
            )
            .is_none()
        );
    }
}
