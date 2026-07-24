use glam::Vec3;

use super::input::MovementInput;
use crate::application::core::player::camera::Camera;

pub const WALK_SPEED_BLOCKS_PER_SECOND: f32 = 4.3;

pub(super) fn desired_velocity(camera: &Camera, input: &MovementInput) -> Vec3 {
    let direction = camera.horizontal_forward() * axis(input.forward, input.backward)
        + camera.right() * axis(input.right, input.left);
    direction.try_normalize().unwrap_or(Vec3::ZERO) * WALK_SPEED_BLOCKS_PER_SECOND
}

fn axis(positive: bool, negative: bool) -> f32 {
    f32::from(positive) - f32::from(negative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_input_does_not_exceed_walk_speed() {
        let camera = Camera::new(16.0 / 9.0);
        let input = MovementInput {
            forward: true,
            right: true,
            ..Default::default()
        };

        assert!((desired_velocity(&camera, &input).length() - 4.3).abs() < f32::EPSILON);
    }
}
