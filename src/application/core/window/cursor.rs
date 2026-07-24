use winit::window::{CursorGrabMode, Window};

pub fn capture(window: &Window) {
    window
        .set_cursor_grab(CursorGrabMode::Locked)
        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
        .expect("failed to capture the cursor");
    window.set_cursor_visible(false);
}

pub fn release(window: &Window) {
    window
        .set_cursor_grab(CursorGrabMode::None)
        .expect("failed to release the cursor");
    window.set_cursor_visible(true);
}
