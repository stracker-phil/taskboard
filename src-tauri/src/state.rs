//! `AppState`: the stores Tauri manages, opened from the config folder.

use std::path::Path;
use std::sync::Arc;

use crate::connection::JiraAccess;
use crate::db::Db;
use crate::error::AppResult;
use crate::jira::Limiter;
use crate::launch::LaunchQueue;
use crate::nav::NavStore;
use crate::secrets::Keychain;
use crate::store::keybindings::KeybindingsStore;
use crate::store::settings::SettingsStore;
use crate::store::ui_state::UiStateStore;

pub struct AppState {
    pub settings: SettingsStore,
    pub keybindings: KeybindingsStore,
    pub ui_state: UiStateStore,
    pub launches: LaunchQueue,
    pub db: Db,
    pub jira: JiraAccess,
    pub nav: NavStore,
}

impl AppState {
    pub fn open(config_dir: &Path) -> AppResult<Self> {
        Ok(Self {
            settings: SettingsStore::open(config_dir),
            keybindings: KeybindingsStore::open(config_dir),
            ui_state: UiStateStore::open(config_dir),
            launches: LaunchQueue::default(),
            db: Db::open(config_dir)?,
            jira: JiraAccess::new(
                Arc::new(Keychain::new(crate::APP_NAME)),
                Arc::new(Limiter::default()),
            ),
            nav: NavStore::open(config_dir),
        })
    }
}
