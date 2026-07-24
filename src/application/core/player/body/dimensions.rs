pub(crate) const BODY_HEIGHT: f32 = 1.90;
pub(crate) const BODY_MAXIMUM_WIDTH: f32 = 0.98;

pub(super) const LEG_SIZE: [f32; 3] = [0.22, 0.74, 0.28];
pub(super) const LEG_X: f32 = 0.15;
pub(super) const LEG_CENTER_Y: f32 = LEG_SIZE[1] * 0.5;

pub(super) const TORSO_SIZE: [f32; 3] = [0.62, 0.62, 0.32];
pub(super) const TORSO_CENTER_Y: f32 = LEG_SIZE[1] + TORSO_SIZE[1] * 0.5;

pub(super) const ARM_SIZE: [f32; 3] = [0.18, 0.64, 0.22];
pub(super) const ARM_X: f32 = 0.40;
pub(super) const ARM_CENTER_Y: f32 = TORSO_CENTER_Y;

pub(super) const HEAD_RADIUS: f32 = 0.25;
pub(super) const HEAD_CENTER_Y: f32 = BODY_HEIGHT - HEAD_RADIUS;

const _: () = assert!(BODY_HEIGHT < 2.0);
const _: () = assert!(BODY_MAXIMUM_WIDTH < 1.0);
