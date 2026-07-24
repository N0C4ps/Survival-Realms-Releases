#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderDistance(u32);

impl RenderDistance {
    pub const DEFAULT: Self = Self(8);

    pub const fn new(chunks: u32) -> Self {
        assert!(chunks > 0, "render distance must be at least one chunk");
        Self(chunks)
    }

    pub const fn chunks(self) -> u32 {
        self.0
    }
}

impl Default for RenderDistance {
    fn default() -> Self {
        Self::DEFAULT
    }
}
