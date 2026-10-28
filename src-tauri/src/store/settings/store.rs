//! `settings.json` on disk.

use std::path::Path;

use super::Settings;
use crate::error::AppResult;
use crate::store::json_store::JsonStore;

pub const FILE_NAME: &str = "settings.json";

#[derive(Clone)]
pub struct SettingsStore {
    store: JsonStore<Settings>,
}

impl SettingsStore {
    pub fn open(dir: &Path) -> Self {
        Self {
            store: JsonStore::open_with(dir.join(FILE_NAME), Settings::repaired),
        }
    }

    pub fn get(&self) -> Settings {
        self.store.read(Settings::clone)
    }

    /// Replaces the settings the page owns and returns the settings before and after. The connection
    /// and the sync settings stay: they change only through `edit`, after validation.
    pub async fn update(&self, settings: Settings) -> AppResult<(Settings, Settings)> {
        self.store
            .update_async(move |current| {
                let mut next = settings;
                next.connection = current.connection.clone();
                next.sync = current.sync.clone();
                let previous = std::mem::replace(current, next);
                (previous, current.clone())
            })
            .await
    }

    /// Changes the settings in place and returns what was saved.
    pub async fn edit(
        &self,
        change: impl FnOnce(&mut Settings) + Send + 'static,
    ) -> AppResult<Settings> {
        self.store
            .update_async(move |current| {
                change(current);
                current.clone()
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::settings::{Connection, SyncSettings};
    use crate::test_support::temp_config_dir;
    use std::fs;

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn without_a_file_the_defaults_load_and_nothing_is_created() {
        let dir = temp_config_dir();
        assert_eq!(SettingsStore::open(dir.path()).get(), Settings::default());
        assert!(file_names(dir.path()).is_empty());
    }

    #[test]
    fn unknown_keys_load_and_the_file_is_not_quarantined() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"fromTheFuture": {"enabled": true}}"#,
        )
        .unwrap();

        assert_eq!(SettingsStore::open(dir.path()).get(), Settings::default());
        assert_eq!(file_names(dir.path()), vec![FILE_NAME.to_string()]);
    }

    #[test]
    fn a_corrupt_file_loads_defaults_and_is_moved_to_a_backup() {
        let dir = temp_config_dir();
        fs::write(dir.path().join(FILE_NAME), "{ not json").unwrap();

        assert_eq!(SettingsStore::open(dir.path()).get(), Settings::default());

        let names = file_names(dir.path());
        assert_eq!(names.len(), 1);
        assert!(
            names[0].starts_with("settings.json.") && names[0].ends_with(".corrupt"),
            "{names:?}"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(&names[0])).unwrap(),
            "{ not json"
        );
    }

    #[tokio::test]
    async fn update_returns_previous_and_saved_and_persists_across_reopen() {
        let dir = temp_config_dir();
        let store = SettingsStore::open(dir.path());

        let (previous, saved) = store.update(Settings::default()).await.unwrap();

        assert_eq!(previous, Settings::default());
        assert_eq!(saved, Settings::default());
        assert_eq!(store.get(), saved);
        assert!(dir.path().join(FILE_NAME).exists());
        assert_eq!(SettingsStore::open(dir.path()).get(), saved);
    }

    fn connected() -> Connection {
        Connection {
            site_url: "https://example.atlassian.net/".into(),
            email: "alex.morgan@example.com".into(),
            display_name: Some("Alex Morgan".into()),
            ..Connection::default()
        }
    }

    async fn store_with_connection_and_sync(dir: &Path) -> SettingsStore {
        let store = SettingsStore::open(dir);
        store
            .edit(|s| {
                s.connection = connected();
                s.sync.focused_minutes = 10;
                s.sync.scope_override = Some("project = NW".into());
            })
            .await
            .unwrap();
        store
    }

    #[tokio::test]
    async fn update_keeps_the_stored_connection_and_sync_settings() {
        let dir = temp_config_dir();
        let store = store_with_connection_and_sync(dir.path()).await;
        let stored = store.get();

        let (previous, saved) = store.update(Settings::default()).await.unwrap();

        assert_eq!(previous, stored);
        assert_eq!(saved, stored);
        assert_eq!(saved.connection, connected());
        assert_eq!(saved.sync.focused_minutes, 10);
        assert_eq!(SettingsStore::open(dir.path()).get(), stored);
    }

    #[tokio::test]
    async fn edit_changes_the_settings_returns_them_and_persists_them() {
        let dir = temp_config_dir();
        let store = SettingsStore::open(dir.path());

        let saved = store.edit(|s| s.sync.keep_done_days = 30).await.unwrap();

        let expected = Settings {
            sync: SyncSettings {
                keep_done_days: 30,
                ..SyncSettings::default()
            },
            ..Settings::default()
        };
        assert_eq!(saved, expected);
        assert_eq!(store.get(), expected);
        assert_eq!(SettingsStore::open(dir.path()).get(), expected);
    }

    #[test]
    fn an_out_of_range_value_in_the_file_is_repaired_and_the_rest_stays_default() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"sync":{"focusedMinutes":0}}"#,
        )
        .unwrap();

        let settings = SettingsStore::open(dir.path()).get();

        assert_eq!(
            settings,
            Settings {
                sync: SyncSettings {
                    focused_minutes: 1,
                    ..SyncSettings::default()
                },
                ..Settings::default()
            }
        );
    }
}
