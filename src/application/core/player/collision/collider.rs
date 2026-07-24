use glam::{BVec3, IVec3, Vec3};

use crate::application::core::{blocks::BlockRegistry, world::World};

use super::{aabb::Aabb, result::CollisionResult};

const PLAYER_WIDTH: f32 = 0.6;
const PLAYER_HEIGHT: f32 = super::super::body::BODY_HEIGHT;
const HALF_PLAYER_WIDTH: f32 = PLAYER_WIDTH * 0.5;
const COLLISION_EPSILON: f32 = 0.000_01;
const PLACEMENT_TOLERANCE: f32 = 0.001;

#[derive(Default)]
pub(crate) struct PlayerCollider;

impl PlayerCollider {
    pub fn overlaps_block(&self, player_position: Vec3, block_position: IVec3) -> bool {
        let minimum = block_position.as_vec3();
        player_bounds(player_position).intersects_with_tolerance(
            &Aabb {
                minimum,
                maximum: minimum + Vec3::ONE,
            },
            PLACEMENT_TOLERANCE,
        )
    }

    pub fn move_and_collide(
        &self,
        position: Vec3,
        displacement: Vec3,
        world: &World,
        registry: &BlockRegistry,
    ) -> CollisionResult {
        let (position, blocked_x) =
            resolve_axis(position, displacement.x, Axis::X, world, registry);
        let (position, blocked_y) =
            resolve_axis(position, displacement.y, Axis::Y, world, registry);
        let (position, blocked_z) =
            resolve_axis(position, displacement.z, Axis::Z, world, registry);

        CollisionResult {
            position,
            blocked_axes: BVec3::new(blocked_x, blocked_y, blocked_z),
            grounded: blocked_y && displacement.y < 0.0,
        }
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

fn resolve_axis(
    position: Vec3,
    displacement: f32,
    axis: Axis,
    world: &World,
    registry: &BlockRegistry,
) -> (Vec3, bool) {
    if displacement == 0.0 {
        return (position, false);
    }

    let mut candidate = position;
    let candidate_axis = axis_value(candidate, axis) + displacement;
    set_axis(&mut candidate, axis, candidate_axis);
    let bounds = player_bounds(position).union(&player_bounds(candidate));
    let minimum = bounds.block_minimum();
    let maximum = bounds.block_maximum();
    let mut corrected = axis_value(candidate, axis);
    let mut blocked = false;

    for x in minimum.x..=maximum.x {
        for y in minimum.y..=maximum.y {
            for z in minimum.z..=maximum.z {
                let block_position = IVec3::new(x, y, z);
                if !registry
                    .get(world.block(block_position))
                    .properties()
                    .is_solid()
                {
                    continue;
                }

                blocked = true;
                corrected = collision_boundary(block_position, axis, displacement, corrected);
            }
        }
    }

    if blocked {
        set_axis(&mut candidate, axis, corrected);
    }
    (candidate, blocked)
}

fn collision_boundary(block: IVec3, axis: Axis, movement: f32, current: f32) -> f32 {
    let block_coordinate = match axis {
        Axis::X => block.x as f32,
        Axis::Y => block.y as f32,
        Axis::Z => block.z as f32,
    };

    if movement > 0.0 {
        let player_extent = match axis {
            Axis::X | Axis::Z => HALF_PLAYER_WIDTH,
            Axis::Y => PLAYER_HEIGHT,
        };
        current.min(block_coordinate - player_extent - COLLISION_EPSILON)
    } else {
        let player_extent = match axis {
            Axis::X | Axis::Z => HALF_PLAYER_WIDTH,
            Axis::Y => 0.0,
        };
        current.max(block_coordinate + 1.0 + player_extent + COLLISION_EPSILON)
    }
}

fn player_bounds(position: Vec3) -> Aabb {
    Aabb {
        minimum: Vec3::new(
            position.x - HALF_PLAYER_WIDTH,
            position.y,
            position.z - HALF_PLAYER_WIDTH,
        ),
        maximum: Vec3::new(
            position.x + HALF_PLAYER_WIDTH,
            position.y + PLAYER_HEIGHT,
            position.z + HALF_PLAYER_WIDTH,
        ),
    }
}

fn axis_value(vector: Vec3, axis: Axis) -> f32 {
    match axis {
        Axis::X => vector.x,
        Axis::Y => vector.y,
        Axis::Z => vector.z,
    }
}

fn set_axis(vector: &mut Vec3, axis: Axis, value: f32) {
    match axis {
        Axis::X => vector.x = value,
        Axis::Y => vector.y = value,
        Axis::Z => vector.z = value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::core::{blocks::BlockId, world::ChunkFlags};

    fn collision_world(blocks: &[IVec3]) -> (World, BlockRegistry) {
        let mut world = World::default();
        for &position in blocks {
            world.set_block(position, BlockId::STONE);
        }
        for (_, chunk) in world.chunks() {
            assert!(chunk.flags().contains(ChunkFlags::LOADED));
        }
        (world, BlockRegistry::builtin())
    }

    #[test]
    fn stops_at_a_wall_without_entering_the_block() {
        let (world, registry) = collision_world(&[IVec3::new(1, 2, 0), IVec3::new(1, 3, 0)]);
        let result =
            PlayerCollider.move_and_collide(Vec3::new(0.0, 2.0, 0.0), Vec3::X, &world, &registry);

        assert!(result.blocked_axes.x);
        assert!((result.position.x - 0.7).abs() < 0.000_1);
    }

    #[test]
    fn cannot_move_down_through_the_floor() {
        let (world, registry) = collision_world(&[IVec3::new(0, 1, 0)]);
        let result =
            PlayerCollider.move_and_collide(Vec3::new(0.0, 2.0, 0.0), -Vec3::Y, &world, &registry);

        assert!(result.blocked_axes.y);
        assert!((result.position.y - 2.0).abs() < 0.000_1);
    }

    #[test]
    fn cannot_move_up_through_a_ceiling() {
        let (world, registry) = collision_world(&[IVec3::new(0, 4, 0)]);
        let result =
            PlayerCollider.move_and_collide(Vec3::new(0.0, 2.0, 0.0), Vec3::Y, &world, &registry);

        assert!(result.blocked_axes.y);
        assert!((result.position.y - 2.1).abs() < 0.000_1);
    }

    #[test]
    fn fast_fall_cannot_tunnel_through_the_floor() {
        let (world, registry) = collision_world(&[IVec3::new(0, 1, 0)]);
        let result = PlayerCollider.move_and_collide(
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(0.0, -20.0, 0.0),
            &world,
            &registry,
        );

        assert!(result.grounded);
        assert!((result.position.y - 2.0).abs() < 0.000_1);
    }

    #[test]
    fn placement_allows_a_block_touching_the_players_head() {
        let collider = PlayerCollider;
        let grounded_position = Vec3::new(0.0, 2.0 + COLLISION_EPSILON, 0.0);

        assert!(!collider.overlaps_block(grounded_position, IVec3::new(0, 4, 0)));
        assert!(collider.overlaps_block(grounded_position, IVec3::new(0, 3, 0)));
    }
}
