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
    use crate::pins::PinDuration;
    use crate::store::settings::{
        Appearance, Behaviour, Connection, MAX_SEEN_AFTER_SECONDS, SyncSettings,
    };
    use crate::test_support::temp_config_dir;
    use crate::views::IssueOpening;
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
    fn a_file_without_appearance_reads_as_system() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"sync":{"focusedMinutes":10}}"#,
        )
        .unwrap();

        let settings = SettingsStore::open(dir.path()).get();

        assert_eq!(settings.appearance, Appearance::System);
        assert_eq!(settings.sync.focused_minutes, 10);
    }

    #[tokio::test]
    async fn each_appearance_round_trips_through_the_store_and_the_file() {
        for (appearance, name) in [
            (Appearance::System, "system"),
            (Appearance::Light, "light"),
            (Appearance::Dark, "dark"),
        ] {
            let dir = temp_config_dir();
            let store = SettingsStore::open(dir.path());

            let saved = store
                .edit(move |s| s.appearance = appearance)
                .await
                .unwrap();

            assert_eq!(saved.appearance, appearance, "{name}");
            assert_eq!(store.get().appearance, appearance, "{name}");
            assert_eq!(
                SettingsStore::open(dir.path()).get().appearance,
                appearance,
                "{name}"
            );
            let file: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(dir.path().join(FILE_NAME)).unwrap())
                    .unwrap();
            assert_eq!(file["appearance"], serde_json::json!(name), "{name}");
        }
    }

    #[test]
    fn an_unknown_appearance_in_the_file_reads_as_system_and_keeps_the_rest() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"appearance":"sepia","sync":{"focusedMinutes":10}}"#,
        )
        .unwrap();

        let settings = SettingsStore::open(dir.path()).get();

        assert_eq!(
            settings,
            Settings {
                appearance: Appearance::System,
                sync: SyncSettings {
                    focused_minutes: 10,
                    ..SyncSettings::default()
                },
                ..Settings::default()
            }
        );
        assert_eq!(file_names(dir.path()), vec![FILE_NAME.to_string()]);
    }

    #[tokio::test]
    async fn changing_the_appearance_keeps_the_connection_and_sync_settings() {
        let dir = temp_config_dir();
        let store = store_with_connection_and_sync(dir.path()).await;
        let stored = store.get();

        let saved = store
            .edit(|s| s.appearance = Appearance::Dark)
            .await
            .unwrap();

        assert_eq!(
            saved,
            Settings {
                appearance: Appearance::Dark,
                ..stored
            }
        );
        assert_eq!(saved.connection, connected());
        assert_eq!(saved.sync.focused_minutes, 10);
        assert_eq!(saved.sync.scope_override.as_deref(), Some("project = NW"));
        assert_eq!(SettingsStore::open(dir.path()).get(), saved);
    }

    #[test]
    fn settings_serialize_with_the_camel_case_appearance() {
        let settings = Settings {
            appearance: Appearance::Light,
            ..Settings::default()
        };

        assert_eq!(
            serde_json::to_value(&settings).unwrap()["appearance"],
            serde_json::json!("light")
        );
    }

    #[test]
    fn a_file_without_behaviour_reads_as_two_seconds_and_until_unpinned() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"sync":{"focusedMinutes":10}}"#,
        )
        .unwrap();

        let settings = SettingsStore::open(dir.path()).get();

        assert_eq!(
            settings.behaviour,
            Behaviour {
                seen_after_seconds: 2,
                pin_default: PinDuration::UntilUnpinned,
                open_issues_in: IssueOpening::Preview,
            }
        );
    }

    #[test]
    fn the_default_behaviour_opens_issues_in_the_preview() {
        assert_eq!(Behaviour::default().open_issues_in, IssueOpening::Preview);
    }

    #[test]
    fn a_behaviour_serializes_the_opening_choice_as_open_issues_in() {
        for (opening, text) in [
            (IssueOpening::Preview, "preview"),
            (IssueOpening::Full, "full"),
        ] {
            let behaviour = Behaviour {
                open_issues_in: opening,
                ..Behaviour::default()
            };

            assert_eq!(
                serde_json::to_value(behaviour).unwrap()["openIssuesIn"],
                serde_json::json!(text),
                "{text}"
            );
        }
    }

    #[test]
    fn a_behaviour_in_the_file_without_the_opening_choice_reads_as_preview() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"behaviour":{"seenAfterSeconds":4,"pinDefault":"week"}}"#,
        )
        .unwrap();

        let behaviour = SettingsStore::open(dir.path()).get().behaviour;

        assert_eq!(
            behaviour,
            Behaviour {
                seen_after_seconds: 4,
                pin_default: PinDuration::Week,
                open_issues_in: IssueOpening::Preview,
            }
        );
    }

    #[test]
    fn the_opening_choice_in_the_file_is_read_and_an_unknown_one_reads_as_preview() {
        for (text, expected) in [
            ("full", IssueOpening::Full),
            ("preview", IssueOpening::Preview),
            ("sidebar", IssueOpening::Preview),
        ] {
            let dir = temp_config_dir();
            fs::write(
                dir.path().join(FILE_NAME),
                format!(r#"{{"behaviour":{{"openIssuesIn":"{text}"}}}}"#),
            )
            .unwrap();

            let behaviour = SettingsStore::open(dir.path()).get().behaviour;

            assert_eq!(behaviour.open_issues_in, expected, "{text}");
        }
    }

    #[tokio::test]
    async fn the_behaviour_round_trips_through_the_store_and_the_file() {
        let dir = temp_config_dir();
        let store = SettingsStore::open(dir.path());
        let behaviour = Behaviour {
            seen_after_seconds: 5,
            pin_default: PinDuration::TwoDays,
            open_issues_in: IssueOpening::Full,
        };

        let saved = store.edit(move |s| s.behaviour = behaviour).await.unwrap();

        assert_eq!(saved.behaviour, behaviour);
        assert_eq!(store.get().behaviour, behaviour);
        assert_eq!(SettingsStore::open(dir.path()).get().behaviour, behaviour);
        let file: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join(FILE_NAME)).unwrap()).unwrap();
        assert_eq!(
            file["behaviour"],
            serde_json::json!({"seenAfterSeconds": 5, "pinDefault": "twoDays", "openIssuesIn": "full"})
        );
    }

    #[test]
    fn a_seen_delay_above_the_maximum_in_the_file_loads_as_the_maximum() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"behaviour":{"seenAfterSeconds":99,"pinDefault":"week"}}"#,
        )
        .unwrap();

        let behaviour = SettingsStore::open(dir.path()).get().behaviour;

        assert_eq!(
            behaviour,
            Behaviour {
                seen_after_seconds: MAX_SEEN_AFTER_SECONDS,
                pin_default: PinDuration::Week,
                open_issues_in: IssueOpening::Preview,
            }
        );
        assert_eq!(MAX_SEEN_AFTER_SECONDS, 10);
    }

    #[test]
    fn an_unknown_pin_default_in_the_file_reads_as_until_unpinned_and_keeps_the_rest() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"behaviour":{"seenAfterSeconds":4,"pinDefault":"fortnight"},"sync":{"focusedMinutes":10}}"#,
        )
        .unwrap();

        let settings = SettingsStore::open(dir.path()).get();

        assert_eq!(
            settings,
            Settings {
                behaviour: Behaviour {
                    seen_after_seconds: 4,
                    pin_default: PinDuration::UntilUnpinned,
                    open_issues_in: IssueOpening::Preview,
                },
                sync: SyncSettings {
                    focused_minutes: 10,
                    ..SyncSettings::default()
                },
                ..Settings::default()
            }
        );
        assert_eq!(file_names(dir.path()), vec![FILE_NAME.to_string()]);
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
