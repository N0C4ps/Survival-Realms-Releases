use std::sync::Arc;

use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop, window::Window};

const WINDOW_TITLE: &str = "Survival Realms";
const INITIAL_WIDTH: u32 = 1280;
const INITIAL_HEIGHT: u32 = 720;

pub fn create(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let initial_size = PhysicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT);
    let attributes = Window::default_attributes()
        .with_title(WINDOW_TITLE)
        .with_inner_size(initial_size)
        .with_resizable(true);

    let window = Arc::new(
        event_loop
            .create_window(attributes)
            .expect("failed to create the game window"),
    );
    tracing::info!(
        title = WINDOW_TITLE,
        width = INITIAL_WIDTH,
        height = INITIAL_HEIGHT,
        resizable = true,
        fullscreen = false,
        "window created"
    );
    window
}
