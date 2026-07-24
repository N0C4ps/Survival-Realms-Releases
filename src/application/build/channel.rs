use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum ReleaseChannel {
    Development,
    Snapshot,
    Release,
}

impl ReleaseChannel {
    pub(super) fn current() -> Self {
        match option_env!("SG_RELEASE_CHANNEL") {
            Some("release") => Self::Release,
            Some("snapshot") => Self::Snapshot,
            Some("development") | None if cfg!(debug_assertions) => Self::Development,
            Some("development") | None => Self::Release,
            Some(channel) => panic!("invalid SG_RELEASE_CHANNEL: {channel}"),
        }
    }
}
