//! Taskboard's configuration in `settings.json`. Missing keys read as their defaults, so a hand-edited file
//! with a missing part keeps the rest.

mod store;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use store::{FILE_NAME as SETTINGS_FILE, SettingsStore};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {}
