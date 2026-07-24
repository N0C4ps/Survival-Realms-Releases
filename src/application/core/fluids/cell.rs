use super::FluidKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FluidCell {
    pub(super) kind: FluidKind,
    pub(super) level: u8,
    pub(super) falling: bool,
    pub(super) source: bool,
    pub(super) revision: u64,
}
