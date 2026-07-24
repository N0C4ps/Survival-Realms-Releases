use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub(crate) struct BodyPose {
    pub position: Vec3,
    pub forward: Vec3,
}

impl BodyPose {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "constructed by the future multiplayer entity stream"
        )
    )]
    pub(crate) fn new(position: Vec3, forward: Vec3) -> Self {
        Self {
            position,
            forward: Vec3::new(forward.x, 0.0, forward.z)
                .try_normalize()
                .unwrap_or(Vec3::NEG_Z),
        }
    }

    pub(crate) fn yaw(self) -> f32 {
        (-self.forward.x).atan2(-self.forward.z)
    }
}
