#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StreamingUpdate {
    scheduled: usize,
    loaded: usize,
    unloaded: usize,
}

impl StreamingUpdate {
    pub(super) const fn scheduled(scheduled: usize, unloaded: usize) -> Self {
        Self {
            scheduled,
            loaded: 0,
            unloaded,
        }
    }

    pub(super) const fn loaded(loaded: usize) -> Self {
        Self {
            scheduled: 0,
            loaded,
            unloaded: 0,
        }
    }

    pub const fn scheduled_count(self) -> usize {
        self.scheduled
    }

    pub const fn loaded_count(self) -> usize {
        self.loaded
    }

    pub const fn unloaded_count(self) -> usize {
        self.unloaded
    }

    pub const fn world_changed(self) -> bool {
        self.loaded > 0 || self.unloaded > 0
    }
}
