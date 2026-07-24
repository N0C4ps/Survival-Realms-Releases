#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum PauseMenuPage {
    #[default]
    Main,
    Options,
    Graphics,
    MouseAndKeyboard,
}

#[derive(Default)]
pub(crate) struct PauseMenuState {
    pub(super) page: PauseMenuPage,
    pub(super) selected: usize,
}

impl PauseMenuState {
    pub(crate) fn reset(&mut self) {
        self.page = PauseMenuPage::Main;
        self.selected = 0;
    }
}
