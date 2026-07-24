use winit::keyboard::{KeyCode, PhysicalKey};

use crate::application::core::blocks::BlockId;

pub(crate) const SLOTS: [BlockId; 8] = [
    BlockId::DIRT,
    BlockId::STONE,
    BlockId::COBBLESTONE,
    BlockId::PLANKS,
    BlockId::CLAY,
    BlockId::SAND,
    BlockId::GRAVEL,
    BlockId::WOOD_LOG,
];

pub(super) fn index_for_key(key: PhysicalKey) -> Option<usize> {
    match key {
        PhysicalKey::Code(KeyCode::Digit1) => Some(0),
        PhysicalKey::Code(KeyCode::Digit2) => Some(1),
        PhysicalKey::Code(KeyCode::Digit3) => Some(2),
        PhysicalKey::Code(KeyCode::Digit4) => Some(3),
        PhysicalKey::Code(KeyCode::Digit5) => Some(4),
        PhysicalKey::Code(KeyCode::Digit6) => Some(5),
        PhysicalKey::Code(KeyCode::Digit7) => Some(6),
        PhysicalKey::Code(KeyCode::Digit8) => Some(7),
        _ => None,
    }
}
