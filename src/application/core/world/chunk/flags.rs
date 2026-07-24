use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct ChunkFlags: u8 {
        const DIRTY = 1 << 0;
        const MESHED = 1 << 1;
        const LOADED = 1 << 2;
        const LIGHT_SOURCES = 1 << 3;
        const GRASS_TICKS = 1 << 4;
        const FLUID_SOURCES = 1 << 5;
    }
}
