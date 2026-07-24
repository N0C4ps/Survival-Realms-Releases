use winit::window::{Fullscreen, Window};

pub fn toggle(window: &Window) {
    if window.fullscreen().is_some() {
        window.set_fullscreen(None);
        tracing::info!("fullscreen disabled");
    } else {
        window.set_fullscreen(Some(Fullscreen::Borderless(window.current_monitor())));
        tracing::info!("fullscreen enabled");
    }
}
