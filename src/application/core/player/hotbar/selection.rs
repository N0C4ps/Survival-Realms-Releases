use winit::{event::ElementState, keyboard::PhysicalKey};

use crate::application::core::blocks::BlockId;

use super::slots;

#[derive(Default)]
pub(crate) struct HotbarSelection {
    selected_index: usize,
}

impl HotbarSelection {
    pub fn process_keyboard(&mut self, key: PhysicalKey, state: ElementState) -> bool {
        if state != ElementState::Pressed {
            return false;
        }
        let Some(index) = slots::index_for_key(key) else {
            return false;
        };
        let changed = self.selected_index != index;
        self.selected_index = index;
        if changed {
            self.log_selection();
        }
        true
    }

    pub fn process_scroll(&mut self, vertical_delta: f32) -> bool {
        if vertical_delta == 0.0 {
            return false;
        }
        if vertical_delta > 0.0 {
            self.selected_index =
                (self.selected_index + slots::SLOTS.len() - 1) % slots::SLOTS.len();
        } else {
            self.selected_index = (self.selected_index + 1) % slots::SLOTS.len();
        }
        self.log_selection();
        true
    }

    pub const fn selected(&self) -> BlockId {
        slots::SLOTS[self.selected_index]
    }

    pub const fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn log_selection(&self) {
        tracing::info!(
            slot = self.selected_index + 1,
            block_id = self.selected().value(),
            "hotbar block selected"
        );
    }
}

#[cfg(test)]
mod tests {
    use winit::keyboard::KeyCode;

    use super::*;

    #[test]
    fn number_keys_select_all_eight_slots() {
        let mut selection = HotbarSelection::default();
        let choices = [
            (KeyCode::Digit1, BlockId::DIRT),
            (KeyCode::Digit2, BlockId::STONE),
            (KeyCode::Digit3, BlockId::COBBLESTONE),
            (KeyCode::Digit4, BlockId::PLANKS),
            (KeyCode::Digit5, BlockId::CLAY),
            (KeyCode::Digit6, BlockId::SAND),
            (KeyCode::Digit7, BlockId::GRAVEL),
            (KeyCode::Digit8, BlockId::WOOD_LOG),
        ];

        for (index, (key, expected)) in choices.into_iter().enumerate() {
            assert!(selection.process_keyboard(PhysicalKey::Code(key), ElementState::Pressed));
            assert_eq!(selection.selected(), expected);
            assert_eq!(selection.selected_index(), index);
        }

        assert!(
            !selection.process_keyboard(PhysicalKey::Code(KeyCode::Digit9), ElementState::Pressed)
        );
    }

    #[test]
    fn mouse_wheel_wraps_in_both_directions() {
        let mut selection = HotbarSelection::default();

        assert!(selection.process_scroll(1.0));
        assert_eq!(selection.selected_index(), 7);
        assert!(selection.process_scroll(-1.0));
        assert_eq!(selection.selected_index(), 0);
        assert!(selection.process_scroll(-120.0));
        assert_eq!(selection.selected_index(), 1);
    }
}
