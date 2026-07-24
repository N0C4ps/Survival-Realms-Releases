use glam::Vec3;

use crate::application::core::{blocks::BlockId, world::World};

const FEET_SAMPLE_HEIGHT: f32 = 0.1;
const BODY_SAMPLE_HEIGHT: f32 = 0.9;
const HEAD_SAMPLE_HEIGHT: f32 = 1.8;
const TOP_SAMPLE_HEIGHT: f32 = 1.95;
const BODY_SAMPLE_RADIUS: f32 = 0.28;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Immersion {
    #[default]
    Outside,
    Feet,
    Partial,
    Head,
    Full,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LiquidContact {
    immersion: Immersion,
    camera_liquid: Option<BlockId>,
}

impl LiquidContact {
    pub(crate) fn detect(world: &World, player_position: Vec3) -> Self {
        let samples = [
            sample_layer(world, player_position, FEET_SAMPLE_HEIGHT),
            sample_layer(world, player_position, BODY_SAMPLE_HEIGHT),
            sample_layer(world, player_position, HEAD_SAMPLE_HEIGHT),
            sample_layer(world, player_position, TOP_SAMPLE_HEIGHT),
        ];
        let camera_liquid = sample(world, player_position + Vec3::Y * HEAD_SAMPLE_HEIGHT);
        let immersion = if samples[3].is_some() {
            Immersion::Full
        } else if samples[2].is_some() {
            Immersion::Head
        } else if samples[1].is_some() {
            Immersion::Partial
        } else if samples[0].is_some() {
            Immersion::Feet
        } else {
            Immersion::Outside
        };

        Self {
            immersion,
            camera_liquid,
        }
    }

    pub(crate) const fn immersion(self) -> Immersion {
        self.immersion
    }

    pub(crate) const fn camera_liquid(self) -> Option<BlockId> {
        self.camera_liquid
    }
}

fn sample_layer(world: &World, player_position: Vec3, height: f32) -> Option<BlockId> {
    [
        Vec3::ZERO,
        Vec3::new(-BODY_SAMPLE_RADIUS, 0.0, -BODY_SAMPLE_RADIUS),
        Vec3::new(BODY_SAMPLE_RADIUS, 0.0, -BODY_SAMPLE_RADIUS),
        Vec3::new(-BODY_SAMPLE_RADIUS, 0.0, BODY_SAMPLE_RADIUS),
        Vec3::new(BODY_SAMPLE_RADIUS, 0.0, BODY_SAMPLE_RADIUS),
    ]
    .into_iter()
    .filter_map(|offset| sample(world, player_position + Vec3::Y * height + offset))
    .max_by_key(|block| u8::from(*block == BlockId::LAVA))
}

fn sample(world: &World, point: Vec3) -> Option<BlockId> {
    let position = point.floor().as_ivec3();
    let block = world.block(position);
    if !block.is_liquid() {
        return None;
    }
    let surface = position.y as f32 + world.liquid_height(position)?;
    (point.y < surface).then_some(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    fn water_column(top: i32) -> World {
        let mut world = World::default();
        for y in 0..=top {
            world.set_block(IVec3::new(0, y, 0), BlockId::WATER);
        }
        world
    }

    #[test]
    fn distinguishes_every_player_immersion_stage() {
        let position = Vec3::new(0.5, 1.2, 0.5);
        assert_eq!(
            LiquidContact::detect(&World::default(), position).immersion(),
            Immersion::Outside
        );
        assert_eq!(
            LiquidContact::detect(&water_column(1), position).immersion(),
            Immersion::Feet
        );
        assert_eq!(
            LiquidContact::detect(&water_column(2), position).immersion(),
            Immersion::Partial
        );
        let mut head_world = water_column(3);
        head_world.set_fluid_state(
            IVec3::new(0, 3, 0),
            crate::application::core::world::FluidState::new(8, false),
        );
        assert_eq!(
            LiquidContact::detect(&head_world, position).immersion(),
            Immersion::Head
        );
        assert_eq!(
            LiquidContact::detect(&water_column(3), position).immersion(),
            Immersion::Full
        );
    }

    #[test]
    fn visual_effect_starts_only_when_the_camera_is_under_liquid() {
        let position = Vec3::new(0.5, 1.2, 0.5);
        assert_eq!(
            LiquidContact::detect(&water_column(2), position).camera_liquid(),
            None
        );
        assert_eq!(
            LiquidContact::detect(&water_column(3), position).camera_liquid(),
            Some(BlockId::WATER)
        );
    }
}
