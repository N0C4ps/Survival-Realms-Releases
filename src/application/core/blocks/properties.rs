#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockProperties {
    solid: bool,
    opaque: bool,
    replaceable: bool,
    renderable: bool,
    liquid: bool,
}

impl BlockProperties {
    pub const AIR: Self = Self {
        solid: false,
        opaque: false,
        replaceable: true,
        renderable: false,
        liquid: false,
    };

    pub const SOLID: Self = Self {
        solid: true,
        opaque: true,
        replaceable: false,
        renderable: true,
        liquid: false,
    };

    /// A collidable block rendered with alpha-tested holes, such as leaves.
    pub const CUTOUT: Self = Self {
        solid: true,
        opaque: false,
        replaceable: false,
        renderable: true,
        liquid: false,
    };

    /// A visible fluid block. Fluid simulation is intentionally separate from
    /// the registry; this only establishes its collision and light behaviour.
    pub const LIQUID: Self = Self {
        solid: false,
        opaque: false,
        replaceable: true,
        renderable: true,
        liquid: true,
    };

    pub const fn is_solid(self) -> bool {
        self.solid
    }

    pub const fn is_opaque(self) -> bool {
        self.opaque
    }

    pub const fn is_replaceable(self) -> bool {
        self.replaceable
    }

    pub const fn is_renderable(self) -> bool {
        self.renderable
    }

    pub const fn is_liquid(self) -> bool {
        self.liquid
    }
}
