use std::time::Duration;

use glam::{BVec3, Vec3};
use winit::{event::ElementState, keyboard::PhysicalKey};

use super::{calculation, input::MovementInput, velocity::MovementVelocity};
use crate::application::core::player::camera::Camera;

pub(crate) struct MovementController {
    input: MovementInput,
    velocity: MovementVelocity,
}

impl MovementController {
    pub fn new() -> Self {
        Self {
            input: MovementInput::default(),
            velocity: MovementVelocity::default(),
        }
    }

    pub fn process_keyboard(&mut self, key: PhysicalKey, state: ElementState) {
        self.input.update(key, state);
    }

    pub fn update(&mut self, camera: &Camera, delta_time: Duration, speed_multiplier: f32) -> Vec3 {
        let seconds = delta_time.as_secs_f32();
        self.velocity.update(
            calculation::desired_velocity(camera, &self.input) * speed_multiplier,
            seconds,
        );
        self.velocity.displacement(seconds)
    }

    pub fn handle_collision(&mut self, blocked_axes: BVec3) {
        self.velocity.cancel_blocked_axes(blocked_axes);
    }

    pub fn reset(&mut self) {
        self.input = MovementInput::default();
        self.velocity = MovementVelocity::default();
    }
}
