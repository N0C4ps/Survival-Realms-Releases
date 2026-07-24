use crate::application::core::settings::GameSettings;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PauseMenuAction {
    Resume,
    Save,
    Quit,
    SettingsChanged(GameSettings),
}
