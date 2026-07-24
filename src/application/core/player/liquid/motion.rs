use std::time::Duration;

use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

use super::Immersion;
use crate::application::core::physics::gravity::Gravity;

const LIQUID_GRAVITY_MULTIPLIER: f32 = 0.2;
const VELOCITY_RETAINED_PER_TICK: f32 = 0.8;
const PHYSICS_TICKS_PER_SECOND: f32 = 20.0;
const ASCEND_ACCELERATION: f32 = 16.0;
const DESCEND_ACCELERATION: f32 = 8.0;
const MAXIMUM_ASCEND_SPEED: f32 = 4.25;
const MAXIMUM_DESCEND_SPEED: f32 = -4.0;
const SURFACE_EXIT_SPEED: f32 = 3.8;

#[derive(Default)]
pub(crate) struct LiquidMotion {
    ascend: bool,
    descend: bool,
}

impl LiquidMotion {
    pub(crate) fn process_keyboard(&mut self, key: PhysicalKey, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        match key {
            PhysicalKey::Code(KeyCode::Space) => self.ascend = pressed,
            PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => self.descend = pressed,
            _ => {}
        }
    }

    pub(crate) fn integrate(
        &self,
        vertical_velocity: &mut f32,
        gravity: Gravity,
        delta_time: Duration,
        immersion: Immersion,
    ) -> f32 {
        let seconds = delta_time.as_secs_f32();
        let previous = *vertical_velocity;
        let drag = VELOCITY_RETAINED_PER_TICK.powf(seconds * PHYSICS_TICKS_PER_SECOND);
        let mut next =
            previous * drag + gravity.acceleration() * LIQUID_GRAVITY_MULTIPLIER * seconds;
        if self.ascend {
            next += ASCEND_ACCELERATION * seconds;
            if immersion == Immersion::Feet {
                next = next.max(SURFACE_EXIT_SPEED);
            }
        }
        if self.descend {
            next -= DESCEND_ACCELERATION * seconds;
        }
        next = next.clamp(MAXIMUM_DESCEND_SPEED, MAXIMUM_ASCEND_SPEED);
        *vertical_velocity = next;
        (previous + next) * 0.5 * seconds
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_player_sinks_slowly_and_held_space_swims_upward() {
        let gravity = Gravity::default();
        let mut motion = LiquidMotion::default();
        let mut velocity = 0.0;
        for _ in 0..20 {
            motion.integrate(
                &mut velocity,
                gravity,
                Duration::from_millis(50),
                Immersion::Full,
            );
        }
        assert!(velocity < 0.0);
        assert!(velocity >= MAXIMUM_DESCEND_SPEED);

        motion.process_keyboard(PhysicalKey::Code(KeyCode::Space), ElementState::Pressed);
        for _ in 0..20 {
            motion.integrate(
                &mut velocity,
                gravity,
                Duration::from_millis(50),
                Immersion::Full,
            );
        }
        assert!(velocity > 0.0);
        assert!(velocity <= MAXIMUM_ASCEND_SPEED);
    }

    #[test]
    fn shift_descends_faster_than_passive_sinking() {
        let gravity = Gravity::default();
        let mut passive_velocity = 0.0;
        LiquidMotion::default().integrate(
            &mut passive_velocity,
            gravity,
            Duration::from_millis(50),
            Immersion::Full,
        );

        let mut motion = LiquidMotion::default();
        motion.process_keyboard(PhysicalKey::Code(KeyCode::ShiftLeft), ElementState::Pressed);
        let mut descending_velocity = 0.0;
        motion.integrate(
            &mut descending_velocity,
            gravity,
            Duration::from_millis(50),
            Immersion::Full,
        );

        assert!(descending_velocity < passive_velocity);
    }

    #[test]
    fn held_space_at_the_surface_kicks_the_player_out_of_liquid() {
        let gravity = Gravity::default();
        let mut motion = LiquidMotion::default();
        motion.process_keyboard(PhysicalKey::Code(KeyCode::Space), ElementState::Pressed);
        let mut velocity = 0.0;

        motion.integrate(
            &mut velocity,
            gravity,
            Duration::from_millis(16),
            Immersion::Feet,
        );

        assert!(velocity >= SURFACE_EXIT_SPEED);
    }
}
