#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct LightLevel(u8);

impl LightLevel {
    pub const DARK: Self = Self(0);
    pub const FULL: Self = Self(15);

    pub const fn new(value: u8) -> Self {
        assert!(
            value <= Self::FULL.0,
            "skylight level must be between 0 and 15"
        );
        Self(value)
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn attenuated(self) -> Self {
        Self(self.0.saturating_sub(1))
    }
}

impl From<LightLevel> for u8 {
    fn from(level: LightLevel) -> Self {
        level.value()
    }
}
