use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowId};

use super::state::Runtime;

impl Runtime {
    pub(super) fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        if let Some(renderer) = self.renderer.as_mut() {
            renderer.handle_window_event(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                self.save_level_before_exit();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
                if let Some(player) = self.player.as_mut()
                    && size.width > 0
                    && size.height > 0
                {
                    player.resize_view(size.width as f32 / size.height as f32);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard(event.physical_key, event.state, event.repeat);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_button(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(delta);
            }
            WindowEvent::Focused(true) if !self.paused => {
                crate::application::core::window::capture_cursor(window);
            }
            WindowEvent::RedrawRequested => {
                self.render_next_frame();
                if self.exit_requested {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}
