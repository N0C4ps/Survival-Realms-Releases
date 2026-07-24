const DEFAULT_ACCELERATION: f32 = -30.0;
const DEFAULT_TERMINAL_VELOCITY: f32 = -60.0;

#[derive(Clone, Copy, Debug)]
pub struct Gravity {
    acceleration: f32,
    terminal_velocity: f32,
}

impl Gravity {
    pub const fn new(acceleration: f32, terminal_velocity: f32) -> Self {
        assert!(acceleration < 0.0, "gravity acceleration must be negative");
        assert!(
            terminal_velocity < 0.0,
            "terminal velocity must be negative"
        );
        Self {
            acceleration,
            terminal_velocity,
        }
    }

    pub const fn acceleration(self) -> f32 {
        self.acceleration
    }

    pub fn integrate(self, vertical_velocity: &mut f32, delta_time: f32) -> f32 {
        let previous_velocity = *vertical_velocity;
        let next_velocity =
            (previous_velocity + self.acceleration * delta_time).max(self.terminal_velocity);
        *vertical_velocity = next_velocity;

        (previous_velocity + next_velocity) * 0.5 * delta_time
    }
}

impl Default for Gravity {
    fn default() -> Self {
        Self::new(DEFAULT_ACCELERATION, DEFAULT_TERMINAL_VELOCITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerates_downward_without_exceeding_terminal_velocity() {
        let gravity = Gravity::default();
        let mut velocity = 0.0;

        for _ in 0..600 {
            gravity.integrate(&mut velocity, 1.0 / 60.0);
        }

        assert_eq!(velocity, -60.0);
    }
}
