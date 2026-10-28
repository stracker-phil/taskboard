//! Taskboard's configuration in `settings.json`. Missing keys read as their defaults, so a hand-edited file
//! with a missing part keeps the rest.

mod connection;
mod store;
mod sync;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
}

impl Settings {
    fn repaired(self) -> Self {
        Self {
            sync: self.sync.clamped(),
            ..self
        }
    }
}
