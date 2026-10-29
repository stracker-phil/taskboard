//! Taskboard's configuration in `settings.json`. Missing keys read as their defaults, so a hand-edited file
//! with a missing part keeps the rest.

mod connection;
mod store;
mod sync;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::pins::PinDuration;
use crate::views::IssueOpening;

pub use connection::Connection;
pub use store::{FILE_NAME as SETTINGS_FILE, SettingsStore};
pub use sync::SyncSettings;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Changed only through the connection commands, which validate it.
    pub connection: Connection,
    /// Changed only through the sync commands, which validate it.
    pub sync: SyncSettings,
    pub appearance: Appearance,
    pub behaviour: Behaviour,
}

/// How long an open issue takes to count as seen, the most a delay can be.
pub const MAX_SEEN_AFTER_SECONDS: u8 = 10;

/// How issues and pins behave by default (Settings → General).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Behaviour {
    /// An issue open in the peek or Issue view this long counts as seen; 0 at once.
    pub seen_after_seconds: u8,
    /// What a new pin starts with.
    pub pin_default: PinDuration,
    /// What a click on an issue opens in views that don't choose.
    pub open_issues_in: IssueOpening,
}

impl Default for Behaviour {
    fn default() -> Self {
        Self {
            seen_after_seconds: 2,
            pin_default: PinDuration::UntilUnpinned,
            open_issues_in: IssueOpening::Preview,
        }
    }
}

/// Light or dark: following macOS (the default) or fixed.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Appearance {
    Light,
    Dark,
    /// Also what an unknown value in the file reads as, so the rest of the file stays.
    #[default]
    #[serde(other)]
    System,
}

impl Settings {
    fn repaired(self) -> Self {
        Self {
            sync: self.sync.clamped(),
            behaviour: Behaviour {
                seen_after_seconds: self
                    .behaviour
                    .seen_after_seconds
                    .min(MAX_SEEN_AFTER_SECONDS),
                ..self.behaviour
            },
            ..self
        }
    }
}
