//! `AppState`: the stores Tauri manages, opened from the config folder.

use std::path::Path;
use std::sync::Arc;

use crate::connection::JiraAccess;
use crate::db::Db;
use crate::error::AppResult;
use crate::jira::Limiter;
use crate::jira::gateway::AvatarClient;
use crate::launch::LaunchQueue;
use crate::nav::NavStore;
use crate::secrets::SecretStore;
use crate::store::keybindings::KeybindingsStore;
use crate::store::places::PlaceStore;
use crate::store::settings::SettingsStore;
use crate::store::ui_state::UiStateStore;

pub struct AppState {
    pub settings: SettingsStore,
    pub keybindings: KeybindingsStore,
    pub ui_state: UiStateStore,
    pub launches: LaunchQueue,
    pub db: Db,
    pub jira: JiraAccess,
    pub avatars: AvatarClient,
    pub nav: NavStore,
    pub places: PlaceStore,
}

impl AppState {
    pub fn open(config_dir: &Path) -> AppResult<Self> {
        Ok(Self {
            settings: SettingsStore::open(config_dir),
            keybindings: KeybindingsStore::open(config_dir),
            ui_state: UiStateStore::open(config_dir),
            launches: LaunchQueue::default(),
            db: Db::open(config_dir)?,
            jira: JiraAccess::new(secret_store(), Arc::new(Limiter::default())),
            avatars: AvatarClient::default(),
            nav: NavStore::open(config_dir),
            places: PlaceStore::open(config_dir),
        })
    }
}

/// The release build keeps the token in the Keychain. The debug build only talks to the fake Jira, which takes
/// any token, so it keeps the demo token in memory (ADR 0026): no Keychain prompt after each rebuild, and no
/// owner token can end up in it.
fn secret_store() -> Arc<dyn SecretStore> {
    #[cfg(debug_assertions)]
    {
        use crate::test_support::FakeSecrets;
        use crate::test_support::demo::ME_EMAIL;
        use crate::test_support::fake_jira::DEMO_TOKEN;
        Arc::new(FakeSecrets::with(ME_EMAIL, DEMO_TOKEN))
    }
    #[cfg(not(debug_assertions))]
    {
        Arc::new(crate::secrets::Keychain::new(crate::APP_NAME))
    }
}
