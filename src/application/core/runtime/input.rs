use winit::{
    event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

use super::state::Runtime;
use crate::application::core::{blocks::BlockId, pause_menu::PauseMenuAction, player::BlockAction};

impl Runtime {
    pub(super) fn handle_keyboard(
        &mut self,
        key: PhysicalKey,
        state: ElementState,
        repeated: bool,
    ) {
        if key == PhysicalKey::Code(KeyCode::F3) {
            if state == ElementState::Pressed && !repeated {
                self.debug_overlay.toggle();
            }
            return;
        }
        if key == PhysicalKey::Code(KeyCode::F11) && state == ElementState::Pressed && !repeated {
            if let Some(window) = self.window.as_ref() {
                crate::application::core::window::toggle_fullscreen(window);
            }
            return;
        }
        if key == PhysicalKey::Code(KeyCode::Escape) && state == ElementState::Pressed && !repeated
        {
            self.toggle_pause();
            return;
        }
        if self.paused {
            return;
        }
        if let Some(player) = self.player.as_mut() {
            player.process_keyboard(key, state);
        }
    }

    pub(super) fn handle_device_event(&mut self, event: DeviceEvent) {
        if self.paused {
            return;
        }
        if let DeviceEvent::MouseMotion { delta } = event
            && let Some(player) = self.player.as_mut()
        {
            player.process_mouse_motion(delta);
        }
    }

    pub(super) fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if self.paused {
            return;
        }
        if let Some(player) = self.player.as_mut() {
            player.process_mouse_button(button, state);
        }
    }

    pub(super) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self.paused {
            return;
        }
        if let Some(player) = self.player.as_mut() {
            player.process_mouse_wheel(delta);
        }
    }

    pub(super) fn apply_block_action(&mut self, action: BlockAction) {
        let Some(world) = self.world.as_mut() else {
            return;
        };

        let (changed, broken_block) = match action {
            BlockAction::Break { position } => {
                let current = world.block(position);
                let changed = self.blocks.get(current).properties().is_solid()
                    && world.edit_block(position, BlockId::AIR).is_some();
                (changed, changed.then_some((position, current)))
            }
            BlockAction::Place { position, block } => {
                let current = world.block(position);
                (
                    self.blocks.get(block).properties().is_solid()
                        && self.blocks.get(current).properties().is_replaceable()
                        && world.edit_block(position, block).is_some(),
                    None,
                )
            }
        };

        if changed {
            self.grass.on_block_changed(world, action.position());
            self.falling_blocks
                .on_block_changed(world, &self.blocks, action.position());
            if let Some((position, block)) = broken_block {
                self.particles.emit_block_break(position, block);
            }
            self.level_dirty = true;
            tracing::debug!(position = ?action.position(), action = ?action, "block edited");
        }
    }

    pub(super) fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        if let Some(player) = self.player.as_mut() {
            player.release_inputs();
        }
        if let Some(window) = self.window.as_ref() {
            if self.paused {
                crate::application::core::window::release_cursor(window);
            } else {
                crate::application::core::window::capture_cursor(window);
            }
        }
        if self.paused
            && let Some(renderer) = self.renderer.as_mut()
        {
            renderer.reset_pause_menu();
        }
        self.clock.reset();
        tracing::info!(paused = self.paused, "pause state changed");
    }

    pub(super) fn handle_pause_menu_action(&mut self, action: Option<PauseMenuAction>) {
        match action {
            Some(PauseMenuAction::Resume) => self.toggle_pause(),
            Some(PauseMenuAction::Save) => self.request_level_save(),
            Some(PauseMenuAction::Quit) => {
                self.save_level_before_exit();
                self.exit_requested = true;
            }
            Some(PauseMenuAction::SettingsChanged(mut settings)) => {
                settings.clamp();
                self.settings = settings;
                if let Some(player) = self.player.as_mut() {
                    player.apply_settings(settings);
                }
                let render_distance = crate::application::core::world::RenderDistance::new(
                    u32::from(settings.render_distance),
                );
                self.chunk_streamer.set_render_distance(render_distance);
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.apply_settings(settings);
                }
                self.settings_store.schedule_save(settings);
            }
            None => {}
        }
    }
}
