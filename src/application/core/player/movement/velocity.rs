use glam::{BVec3, Vec3};

const ACCELERATION: f32 = 30.0;
const BRAKING: f32 = 38.0;
const REVERSAL_ACCELERATION: f32 = 48.0;

#[derive(Default)]
pub(super) struct MovementVelocity {
    horizontal: Vec3,
}

impl MovementVelocity {
    pub fn update(&mut self, target: Vec3, delta_time: f32) {
        let response = if target == Vec3::ZERO {
            BRAKING
        } else if self.horizontal.dot(target) < 0.0 {
            REVERSAL_ACCELERATION
        } else {
            ACCELERATION
        };
        self.horizontal = move_towards(self.horizontal, target, response * delta_time);
    }

    pub fn displacement(&self, delta_time: f32) -> Vec3 {
        self.horizontal * delta_time
    }

    pub fn cancel_blocked_axes(&mut self, blocked: BVec3) {
        if blocked.x {
            self.horizontal.x = 0.0;
        }
        if blocked.z {
            self.horizontal.z = 0.0;
        }
    }
}

fn move_towards(current: Vec3, target: Vec3, maximum_delta: f32) -> Vec3 {
    let difference = target - current;
    let distance = difference.length();
    if distance <= maximum_delta || distance == 0.0 {
        target
    } else {
        current + difference / distance * maximum_delta
    }
}
