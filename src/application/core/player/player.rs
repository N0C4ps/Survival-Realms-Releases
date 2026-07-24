use std::time::Duration;

use glam::{IVec3, Vec3};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::PhysicalKey,
};

use super::{
    camera::{Camera, MouseLook},
    collision::{CollisionResult, PlayerCollider},
    hotbar::HotbarSelection,
    liquid::{Immersion, LiquidContact, LiquidMotion},
    movement::MovementController,
    scripts::{BlockAction, BlockInteraction, JumpScript},
    spawn::{CAMERA_EYE_HEIGHT, spawn_position},
};
use crate::application::core::{
    blocks::BlockRegistry,
    physics::{
        gravity::Gravity,
        raycast::{self, RaycastHit},
    },
    settings::GameSettings,
    world::World,
};

const BLOCK_REACH: f32 = 5.0;
const WADING_MOVEMENT_MULTIPLIER: f32 = 0.45;
const PARTIAL_LIQUID_MOVEMENT_MULTIPLIER: f32 = 0.65;
const SUBMERGED_MOVEMENT_MULTIPLIER: f32 = 0.8;

pub struct Player {
    position: Vec3,
    camera: Camera,
    mouse_look: MouseLook,
    movement: MovementController,
    jump: JumpScript,
    gravity: Gravity,
    vertical_velocity: f32,
    grounded: bool,
    collider: PlayerCollider,
    target: Option<RaycastHit>,
    block_interaction: BlockInteraction,
    pending_block_action: Option<BlockAction>,
    hotbar: HotbarSelection,
    liquid_contact: LiquidContact,
    liquid_motion: LiquidMotion,
}

impl Player {
    pub fn new(aspect_ratio: f32, spawn_surface: IVec3) -> Self {
        let position = spawn_position(spawn_surface);
        Self::at_position(aspect_ratio, position)
    }

    pub fn at_position(aspect_ratio: f32, position: Vec3) -> Self {
        let mut camera = Camera::new(aspect_ratio);
        camera.set_position(camera_position(position));

        Self {
            position,
            camera,
            mouse_look: MouseLook::default(),
            movement: MovementController::new(),
            jump: JumpScript::default(),
            gravity: Gravity::default(),
            vertical_velocity: 0.0,
            grounded: true,
            collider: PlayerCollider,
            target: None,
            block_interaction: BlockInteraction::default(),
            pending_block_action: None,
            hotbar: HotbarSelection::default(),
            liquid_contact: LiquidContact::default(),
            liquid_motion: LiquidMotion::default(),
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn targeted_block(&self) -> Option<IVec3> {
        self.target.map(|hit| hit.block)
    }

    pub const fn selected_hotbar_slot(&self) -> usize {
        self.hotbar.selected_index()
    }

    pub(crate) const fn camera_liquid(&self) -> Option<crate::application::core::blocks::BlockId> {
        self.liquid_contact.camera_liquid()
    }

    pub fn process_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        self.block_interaction.process_mouse(button, state);
    }

    pub fn process_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let vertical = match delta {
            MouseScrollDelta::LineDelta(_, vertical) => vertical,
            MouseScrollDelta::PixelDelta(position) => position.y as f32,
        };
        self.hotbar.process_scroll(vertical);
    }

    pub fn take_block_action(&mut self) -> Option<BlockAction> {
        self.pending_block_action.take()
    }

    pub fn release_inputs(&mut self) {
        self.movement.reset();
        self.jump.reset();
        self.liquid_motion.reset();
        self.mouse_look.reset();
        self.block_interaction.reset();
        self.pending_block_action = None;
    }

    fn accept_block_action(&self, action: BlockAction) -> Option<BlockAction> {
        match action {
            BlockAction::Place { position, .. }
                if self.collider.overlaps_block(self.position, position) =>
            {
                None
            }
            _ => Some(action),
        }
    }

    pub fn process_keyboard(&mut self, key: PhysicalKey, state: ElementState) {
        if self.hotbar.process_keyboard(key, state) {
            return;
        }
        self.movement.process_keyboard(key, state);
        self.jump.process_keyboard(key, state);
        self.liquid_motion.process_keyboard(key, state);
    }

    pub fn process_mouse_motion(&mut self, delta: (f64, f64)) {
        self.mouse_look.process_motion(delta);
    }

    pub fn update(&mut self, delta_time: Duration, world: &World, registry: &BlockRegistry) {
        self.mouse_look.update(&mut self.camera);
        self.liquid_contact = LiquidContact::detect(world, self.position);
        let immersion = self.liquid_contact.immersion();
        let in_liquid = !matches!(immersion, Immersion::Outside);

        if !in_liquid
            && self
                .jump
                .apply(self.grounded, &mut self.vertical_velocity, self.gravity)
        {
            self.grounded = false;
        }

        let speed_multiplier = match immersion {
            Immersion::Outside => 1.0,
            Immersion::Feet => WADING_MOVEMENT_MULTIPLIER,
            Immersion::Partial => PARTIAL_LIQUID_MOVEMENT_MULTIPLIER,
            Immersion::Head | Immersion::Full => SUBMERGED_MOVEMENT_MULTIPLIER,
        };
        let mut displacement = self
            .movement
            .update(&self.camera, delta_time, speed_multiplier);
        displacement.y = if in_liquid {
            self.liquid_motion.integrate(
                &mut self.vertical_velocity,
                self.gravity,
                delta_time,
                immersion,
            )
        } else {
            self.gravity
                .integrate(&mut self.vertical_velocity, delta_time.as_secs_f32())
        };
        let collision =
            self.collider
                .move_and_collide(self.position, displacement, world, registry);
        self.apply_collision(collision);
        self.liquid_contact = LiquidContact::detect(world, self.position);
        self.target = raycast::cast_voxels(
            world,
            registry,
            self.camera.position(),
            self.camera.forward(),
            BLOCK_REACH,
        );
        self.pending_block_action = self
            .block_interaction
            .update(delta_time, self.target, self.hotbar.selected())
            .and_then(|action| self.accept_block_action(action));
    }

    pub fn resize_view(&mut self, aspect_ratio: f32) {
        self.camera.set_aspect_ratio(aspect_ratio);
    }

    pub(crate) fn apply_settings(&mut self, settings: GameSettings) {
        self.camera.set_fov_degrees(settings.fov_degrees());
        self.mouse_look
            .set_radians_per_pixel(settings.mouse_radians_per_pixel());
    }

    fn apply_collision(&mut self, collision: CollisionResult) {
        self.position = collision.position;
        self.movement.handle_collision(collision.blocked_axes);
        if collision.blocked_axes.y {
            self.vertical_velocity = 0.0;
        }
        self.grounded = collision.grounded;
        self.camera.set_position(camera_position(self.position));
    }
}

fn camera_position(player_position: Vec3) -> Vec3 {
    player_position + Vec3::Y * CAMERA_EYE_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_above_grass_with_the_camera_at_eye_height() {
        let player = Player::new(16.0 / 9.0, IVec3::new(0, 1, 5));

        assert_eq!(player.position().y, 2.0);
        assert!((player.camera().position().y - 3.8).abs() < 0.000_1);
    }
}
