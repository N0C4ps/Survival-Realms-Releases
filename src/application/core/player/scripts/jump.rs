use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::application::core::physics::gravity::Gravity;

const JUMP_HEIGHT_BLOCKS: f32 = 2.0;

#[derive(Default)]
pub(crate) struct JumpScript {
    space_pressed: bool,
}

impl JumpScript {
    pub fn process_keyboard(&mut self, key: PhysicalKey, state: ElementState) {
        if key != PhysicalKey::Code(KeyCode::Space) {
            return;
        }

        self.space_pressed = state == ElementState::Pressed;
    }

    pub fn apply(&mut self, grounded: bool, velocity_y: &mut f32, gravity: Gravity) -> bool {
        let jumped = self.space_pressed && grounded;
        if jumped {
            *velocity_y = jump_velocity(gravity);
        }
        jumped
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

fn jump_velocity(gravity: Gravity) -> f32 {
    (-2.0 * gravity.acceleration() * JUMP_HEIGHT_BLOCKS).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_reaches_two_blocks_under_default_gravity() {
        let gravity = Gravity::default();
        let velocity = jump_velocity(gravity);
        let height = velocity * velocity / (-2.0 * gravity.acceleration());

        assert!((height - 2.0).abs() < 0.000_001);
    }

    #[test]
    fn held_space_jumps_again_after_landing() {
        let gravity = Gravity::default();
        let mut jump = JumpScript::default();
        let mut velocity = 0.0;

        jump.process_keyboard(PhysicalKey::Code(KeyCode::Space), ElementState::Pressed);

        assert!(jump.apply(true, &mut velocity, gravity));
        velocity = 0.0;
        assert!(jump.apply(true, &mut velocity, gravity));

        jump.process_keyboard(PhysicalKey::Code(KeyCode::Space), ElementState::Released);
        assert!(!jump.apply(true, &mut velocity, gravity));
    }
}
