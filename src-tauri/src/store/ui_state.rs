//! Window layout and the palette's recent searches, kept across restarts (`ui_state.json`). Unknown keys are
//! ignored.

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
    /// The palette's last searches, newest first (`RECENT_SEARCHES` at most); only `remember_search` and
    /// `forget_search` change them.
    pub recent_searches: Vec<String>,
}

/// How many searches the palette remembers.
pub const RECENT_SEARCHES: usize = 10;

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

    /// Replaces the window layout; the recent searches stay as they are, so a page's older copy can't drop one.
    pub async fn set(&self, ui: UiState) -> AppResult<UiState> {
        self.store
            .update_async(move |current| {
                *current = UiState {
                    recent_searches: std::mem::take(&mut current.recent_searches),
                    ..ui
                };
                current.clone()
            })
            .await
    }

    /// Puts the search first, once; the oldest past `RECENT_SEARCHES` goes.
    pub async fn remember_search(&self, text: String) -> AppResult<Vec<String>> {
        let text = text.trim().to_string();
        self.store
            .update_async(move |current| {
                if !text.is_empty() {
                    current.recent_searches.retain(|kept| *kept != text);
                    current.recent_searches.insert(0, text);
                    current.recent_searches.truncate(RECENT_SEARCHES);
                }
                current.recent_searches.clone()
            })
            .await
    }

    pub async fn forget_search(&self, text: String) -> AppResult<Vec<String>> {
        self.store
            .update_async(move |current| {
                current.recent_searches.retain(|kept| *kept != text);
                current.recent_searches.clone()
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

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn without_a_file_the_sidebar_is_visible() {
        let dir = temp_config_dir();
        assert_eq!(
            UiStateStore::open(dir.path()).get(),
            UiState {
                sidebar_hidden: false,
                activity_width: None,
                ..Default::default()
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
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(
            saved,
            UiState {
                sidebar_hidden: true,
                activity_width: None,
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
            }
        );
        assert_eq!(backups(dir.path()), 0);
    }

    #[test]
    fn keys_are_camel_case() {
        let value = serde_json::to_value(UiState {
            sidebar_hidden: true,
            activity_width: Some(320),
            recent_searches: strings(&["checkout"]),
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "sidebarHidden": true,
                "activityWidth": 320,
                "recentSearches": ["checkout"]
            })
        );
    }

    #[tokio::test]
    async fn the_file_on_disk_uses_camel_case_keys() {
        let dir = temp_config_dir();
        UiStateStore::open(dir.path())
            .set(UiState {
                sidebar_hidden: true,
                activity_width: Some(320),
                ..Default::default()
            })
            .await
            .unwrap();

        let raw = fs::read_to_string(dir.path().join("ui_state.json")).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "sidebarHidden": true,
                "activityWidth": 320,
                "recentSearches": []
            })
        );
    }

    #[test]
    fn a_file_without_recent_searches_loads_them_as_empty() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join("ui_state.json"),
            r#"{"sidebarHidden": true, "activityWidth": 300}"#,
        )
        .unwrap();

        assert_eq!(
            UiStateStore::open(dir.path()).get().recent_searches,
            Vec::<String>::new()
        );
        assert_eq!(backups(dir.path()), 0);
    }

    #[test]
    fn recent_searches_are_read_from_the_camel_case_key() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join("ui_state.json"),
            r#"{"recentSearches": ["checkout", "invoice"]}"#,
        )
        .unwrap();

        assert_eq!(
            UiStateStore::open(dir.path()).get().recent_searches,
            strings(&["checkout", "invoice"])
        );
    }

    #[tokio::test]
    async fn a_remembered_search_goes_first_trimmed() {
        let dir = temp_config_dir();
        let store = UiStateStore::open(dir.path());
        store.remember_search("checkout".into()).await.unwrap();

        let recent = store
            .remember_search("  invoice export \n".into())
            .await
            .unwrap();

        assert_eq!(recent, strings(&["invoice export", "checkout"]));
    }

    #[tokio::test]
    async fn a_repeated_search_moves_to_the_front_without_a_duplicate() {
        let dir = temp_config_dir();
        let store = UiStateStore::open(dir.path());
        for text in ["checkout", "invoice", "login"] {
            store.remember_search(text.into()).await.unwrap();
        }

        let recent = store.remember_search(" checkout ".into()).await.unwrap();

        assert_eq!(recent, strings(&["checkout", "login", "invoice"]));
    }

    #[tokio::test]
    async fn only_the_ten_newest_searches_are_kept() {
        let dir = temp_config_dir();
        let store = UiStateStore::open(dir.path());
        let mut recent = Vec::new();
        for n in 1..=12 {
            recent = store.remember_search(format!("search {n}")).await.unwrap();
        }

        let expected: Vec<String> = (3..=12).rev().map(|n| format!("search {n}")).collect();
        assert_eq!(recent.len(), RECENT_SEARCHES);
        assert_eq!(recent, expected);
    }

    #[tokio::test]
    async fn blank_searches_are_ignored() {
        let dir = temp_config_dir();
        let store = UiStateStore::open(dir.path());
        store.remember_search("checkout".into()).await.unwrap();

        for text in ["", "   ", "\t\n"] {
            let recent = store.remember_search(text.into()).await.unwrap();
            assert_eq!(recent, strings(&["checkout"]), "{text:?}");
        }
    }

    #[tokio::test]
    async fn remembered_searches_persist_across_reopen() {
        let dir = temp_config_dir();
        let store = UiStateStore::open(dir.path());
        store.remember_search("checkout".into()).await.unwrap();
        store.remember_search("invoice".into()).await.unwrap();

        assert_eq!(
            UiStateStore::open(dir.path()).get().recent_searches,
            strings(&["invoice", "checkout"])
        );
    }

    #[tokio::test]
    async fn a_forgotten_search_is_removed_and_the_rest_stay_in_order() {
        let dir = temp_config_dir();
        let store = UiStateStore::open(dir.path());
        for text in ["checkout", "invoice", "login"] {
            store.remember_search(text.into()).await.unwrap();
        }

        let recent = store.forget_search("invoice".into()).await.unwrap();

        assert_eq!(recent, strings(&["login", "checkout"]));
        assert_eq!(UiStateStore::open(dir.path()).get().recent_searches, recent);
    }

    #[tokio::test]
    async fn forgetting_an_unknown_search_changes_nothing() {
        let dir = temp_config_dir();
        let store = UiStateStore::open(dir.path());
        store.remember_search("checkout".into()).await.unwrap();

        let recent = store.forget_search("nope".into()).await.unwrap();

        assert_eq!(recent, strings(&["checkout"]));
    }

    #[tokio::test]
    async fn setting_the_layout_keeps_the_stored_recent_searches() {
        let dir = temp_config_dir();
        let store = UiStateStore::open(dir.path());
        store.remember_search("checkout".into()).await.unwrap();
        store.remember_search("invoice".into()).await.unwrap();

        for older in [Vec::new(), strings(&["stale"])] {
            let saved = store
                .set(UiState {
                    sidebar_hidden: true,
                    activity_width: Some(280),
                    recent_searches: older.clone(),
                })
                .await
                .unwrap();

            assert_eq!(
                saved,
                UiState {
                    sidebar_hidden: true,
                    activity_width: Some(280),
                    recent_searches: strings(&["invoice", "checkout"]),
                },
                "{older:?}"
            );
        }
        assert_eq!(
            UiStateStore::open(dir.path()).get().recent_searches,
            strings(&["invoice", "checkout"])
        );
    }
}
