mod cursor;
mod fullscreen;
#[path = "window.rs"]
mod implementation;

pub use cursor::capture as capture_cursor;
pub use cursor::release as release_cursor;
pub use fullscreen::toggle as toggle_fullscreen;
pub use implementation::create;
