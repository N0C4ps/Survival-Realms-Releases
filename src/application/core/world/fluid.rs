#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FluidState {
    level: u8,
    falling: bool,
}

impl FluidState {
    pub(crate) const SOURCE: Self = Self {
        level: 0,
        falling: false,
    };

    pub(crate) const fn new(level: u8, falling: bool) -> Self {
        Self { level, falling }
    }

    pub(crate) const fn level(self) -> u8 {
        self.level
    }

    pub(crate) const fn is_falling(self) -> bool {
        self.falling
    }
}
