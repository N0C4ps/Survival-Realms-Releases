use std::time::Duration;

use glam::IVec3;
use winit::event::{ElementState, MouseButton};

use crate::application::core::{blocks::BlockId, physics::raycast::RaycastHit};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BlockAction {
    Break { position: IVec3 },
    Place { position: IVec3, block: BlockId },
}

const REPEAT_DELAY: Duration = Duration::from_millis(180);

#[derive(Default)]
pub(crate) struct BlockInteraction {
    break_held: bool,
    place_held: bool,
    repeat_remaining: Duration,
    immediate: bool,
}

impl BlockInteraction {
    pub fn process_mouse(&mut self, button: MouseButton, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        let held = match button {
            MouseButton::Left => &mut self.break_held,
            MouseButton::Right => &mut self.place_held,
            _ => return,
        };
        if pressed && !*held {
            self.immediate = true;
        }
        *held = pressed;
        if !self.break_held && !self.place_held {
            self.repeat_remaining = Duration::ZERO;
            self.immediate = false;
        }
    }

    pub fn update(
        &mut self,
        delta_time: Duration,
        hit: Option<RaycastHit>,
        selected_block: BlockId,
    ) -> Option<BlockAction> {
        if !self.break_held && !self.place_held {
            return None;
        }

        let ready = if self.immediate {
            self.immediate = false;
            true
        } else if delta_time >= self.repeat_remaining {
            true
        } else {
            self.repeat_remaining -= delta_time;
            false
        };
        if !ready {
            return None;
        }

        let action = action_for_held_buttons(self.break_held, self.place_held, hit, selected_block);
        if action.is_some() {
            self.repeat_remaining = REPEAT_DELAY;
        }
        action
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl BlockAction {
    fn breaking(hit: RaycastHit) -> Self {
        Self::Break {
            position: hit.block,
        }
    }

    fn placing(hit: RaycastHit, block: BlockId) -> Option<Self> {
        (hit.normal != IVec3::ZERO).then_some(Self::Place {
            position: hit.block + hit.normal,
            block,
        })
    }

    pub const fn position(self) -> IVec3 {
        match self {
            Self::Break { position } | Self::Place { position, .. } => position,
        }
    }
}

fn action_for_held_buttons(
    break_held: bool,
    place_held: bool,
    hit: Option<RaycastHit>,
    selected_block: BlockId,
) -> Option<BlockAction> {
    let hit = hit?;
    if break_held {
        Some(BlockAction::breaking(hit))
    } else if place_held {
        BlockAction::placing(hit, selected_block)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    fn hit() -> RaycastHit {
        RaycastHit {
            block: IVec3::ZERO,
            normal: IVec3::Y,
            distance: 1.0,
            point: Vec3::ZERO,
        }
    }

    #[test]
    fn held_mouse_repeats_only_after_the_delay() {
        let mut interaction = BlockInteraction::default();
        interaction.process_mouse(MouseButton::Left, ElementState::Pressed);

        assert!(
            interaction
                .update(Duration::ZERO, Some(hit()), BlockId::DIRT)
                .is_some()
        );
        assert!(
            interaction
                .update(Duration::from_millis(179), Some(hit()), BlockId::DIRT)
                .is_none()
        );
        assert!(
            interaction
                .update(Duration::from_millis(1), Some(hit()), BlockId::DIRT)
                .is_some()
        );
    }

    #[test]
    fn placement_uses_the_current_hotbar_block() {
        let mut interaction = BlockInteraction::default();
        interaction.process_mouse(MouseButton::Right, ElementState::Pressed);

        let action = interaction
            .update(Duration::ZERO, Some(hit()), BlockId::WOOD)
            .unwrap();

        assert_eq!(
            action,
            BlockAction::Place {
                position: IVec3::Y,
                block: BlockId::WOOD,
            }
        );
    }
}
