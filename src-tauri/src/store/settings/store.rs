//! `settings.json` on disk.

use std::path::Path;

use super::Settings;
use crate::error::AppResult;
use crate::store::json_store::JsonStore;

pub const FILE_NAME: &str = "settings.json";

pub struct SettingsStore {
    store: JsonStore<Settings>,
}

impl SettingsStore {
    pub fn open(dir: &Path) -> Self {
        Self {
            store: JsonStore::open(dir.join(FILE_NAME)),
        }
    }

    pub fn get(&self) -> Settings {
        self.store.read(Settings::clone)
    }

    /// Returns the settings before the change and the settings that were saved.
    pub async fn update(&self, settings: Settings) -> AppResult<(Settings, Settings)> {
        self.store
            .update_async(move |current| {
                let previous = std::mem::replace(current, settings);
                (previous, current.clone())
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
