#[derive(Default)]
pub(crate) struct DebugOverlay {
    visible: bool,
}

impl DebugOverlay {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        tracing::debug!(visible = self.visible, "debug overlay toggled");
    }

    pub const fn is_visible(&self) -> bool {
        self.visible
    }
}
