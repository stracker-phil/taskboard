//! Window layout that survives a restart (`ui_state.json`). Unknown keys are ignored.

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::json_store::JsonStore;
use crate::error::AppResult;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct UiState {
    pub sidebar_hidden: bool,
    /// The Issue view's activity column in points; none is the default width.
    pub activity_width: Option<u32>,
}

pub struct UiStateStore {
    store: JsonStore<UiState>,
}

impl UiStateStore {
    pub fn open(dir: &Path) -> Self {
        Self {
            store: JsonStore::open(dir.join("ui_state.json")),
        }
    }

    pub fn get(&self) -> UiState {
        self.store.read(UiState::clone)
    }

    pub async fn set(&self, ui: UiState) -> AppResult<UiState> {
        self.store
            .update_async(move |current| {
                *current = ui.clone();
                ui
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use std::fs;

    fn backups(dir: &Path) -> usize {
        fs::read_dir(dir)
            .unwrap()
            .filter(|e| {
                let name = e.as_ref().unwrap().file_name();
                name.to_string_lossy().ends_with(".corrupt")
            })
            .count()
    }

    #[test]
    fn without_a_file_the_sidebar_is_visible() {
        let dir = temp_config_dir();
        assert_eq!(
            UiStateStore::open(dir.path()).get(),
            UiState {
                sidebar_hidden: false,
                activity_width: None,
            }
        );
    }

    #[test]
    fn without_a_file_the_activity_width_is_none() {
        let dir = temp_config_dir();
        assert_eq!(UiStateStore::open(dir.path()).get().activity_width, None);
    }

    #[tokio::test]
    async fn saved_state_persists_across_reopen() {
        let dir = temp_config_dir();
        let saved = UiStateStore::open(dir.path())
            .set(UiState {
                sidebar_hidden: true,
                activity_width: None,
            })
            .await
            .unwrap();

        assert_eq!(
            saved,
            UiState {
                sidebar_hidden: true,
                activity_width: None,
            }
        );
        assert_eq!(UiStateStore::open(dir.path()).get(), saved);
    }

    #[tokio::test]
    async fn a_saved_activity_width_comes_back_after_reopening() {
        let dir = temp_config_dir();
        UiStateStore::open(dir.path())
            .set(UiState {
                sidebar_hidden: false,
                activity_width: Some(360),
            })
            .await
            .unwrap();

        assert_eq!(
            UiStateStore::open(dir.path()).get().activity_width,
            Some(360)
        );
    }

    #[test]
    fn an_old_file_with_only_the_sidebar_flag_still_loads() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join("ui_state.json"),
            r#"{"sidebarHidden": true}"#,
        )
        .unwrap();

        assert_eq!(
            UiStateStore::open(dir.path()).get(),
            UiState {
                sidebar_hidden: true,
                activity_width: None,
            }
        );
        assert_eq!(backups(dir.path()), 0);
    }

    #[test]
    fn an_empty_object_loads_defaults_without_quarantining_the_file() {
        let dir = temp_config_dir();
        fs::write(dir.path().join("ui_state.json"), "{}").unwrap();

        assert_eq!(UiStateStore::open(dir.path()).get(), UiState::default());
        assert!(dir.path().join("ui_state.json").exists());
        assert_eq!(backups(dir.path()), 0);
    }

    #[test]
    fn unknown_keys_are_ignored_and_known_ones_still_load() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join("ui_state.json"),
            r#"{"sidebarHidden": true, "futureThing": [1, 2]}"#,
        )
        .unwrap();

        assert_eq!(
            UiStateStore::open(dir.path()).get(),
            UiState {
                sidebar_hidden: true,
                activity_width: None,
            }
        );
        assert_eq!(backups(dir.path()), 0);
    }

    #[test]
    fn keys_are_camel_case() {
        let value = serde_json::to_value(UiState {
            sidebar_hidden: true,
            activity_width: Some(320),
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "sidebarHidden": true, "activityWidth": 320 })
        );
    }

    #[tokio::test]
    async fn the_file_on_disk_uses_camel_case_keys() {
        let dir = temp_config_dir();
        UiStateStore::open(dir.path())
            .set(UiState {
                sidebar_hidden: true,
                activity_width: Some(320),
            })
            .await
            .unwrap();

        let raw = fs::read_to_string(dir.path().join("ui_state.json")).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "sidebarHidden": true, "activityWidth": 320 })
        );
    }
}
