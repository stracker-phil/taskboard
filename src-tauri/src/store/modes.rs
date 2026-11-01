//! Modes that hold in every view until switched off (`modes.json`): Me mode everywhere, Unread only, and where the
//! bookmarks show. Rust applies them to every view query and the menus; the page shows and switches them.

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::json_store::JsonStore;
use crate::error::AppResult;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Modes {
    /// Me mode is on in every view (the Me popup's "All views").
    pub me_everywhere: bool,
    /// Every view and every sidebar count shows only unread issues.
    pub unread_only: bool,
    /// The bookmarks show in a bar above the content instead of in the sidebar.
    pub bookmarks_bar: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    MeEverywhere,
    UnreadOnly,
    BookmarksBar,
}

pub struct ModesStore {
    store: JsonStore<Modes>,
}

impl ModesStore {
    pub fn open(dir: &Path) -> Self {
        Self {
            store: JsonStore::open(dir.join("modes.json")),
        }
    }

    pub fn get(&self) -> Modes {
        self.store.read(|modes| *modes)
    }

    pub async fn set(&self, mode: Mode, on: bool) -> AppResult<Modes> {
        self.store
            .update_async(move |modes| {
                match mode {
                    Mode::MeEverywhere => modes.me_everywhere = on,
                    Mode::UnreadOnly => modes.unread_only = on,
                    Mode::BookmarksBar => modes.bookmarks_bar = on,
                }
                *modes
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use std::fs;

    fn modes(me_everywhere: bool, unread_only: bool) -> Modes {
        Modes {
            me_everywhere,
            unread_only,
            ..Modes::default()
        }
    }

    fn bar_only() -> Modes {
        Modes {
            bookmarks_bar: true,
            ..Modes::default()
        }
    }

    #[tokio::test]
    async fn the_bookmarks_bar_mode_is_independent_of_the_other_modes() {
        let dir = temp_config_dir();
        let store = ModesStore::open(dir.path());

        assert_eq!(
            store.set(Mode::BookmarksBar, true).await.unwrap(),
            bar_only()
        );
        assert_eq!(
            store.set(Mode::UnreadOnly, true).await.unwrap(),
            Modes {
                unread_only: true,
                bookmarks_bar: true,
                ..Modes::default()
            }
        );
        assert_eq!(
            store.set(Mode::UnreadOnly, false).await.unwrap(),
            bar_only()
        );
        assert_eq!(
            store.set(Mode::BookmarksBar, false).await.unwrap(),
            Modes::default()
        );
    }

    #[tokio::test]
    async fn the_bookmarks_bar_mode_persists_across_reopen() {
        let dir = temp_config_dir();
        ModesStore::open(dir.path())
            .set(Mode::BookmarksBar, true)
            .await
            .unwrap();

        assert_eq!(ModesStore::open(dir.path()).get(), bar_only());
    }

    #[test]
    fn a_file_from_before_the_bookmarks_bar_loads_with_the_bar_off() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join("modes.json"),
            r#"{"meEverywhere": true, "unreadOnly": true}"#,
        )
        .unwrap();

        assert_eq!(ModesStore::open(dir.path()).get(), modes(true, true));
    }

    #[test]
    fn the_bookmarks_bar_uses_camel_case_on_the_wire() {
        assert_eq!(
            serde_json::to_value(bar_only()).unwrap(),
            serde_json::json!({
                "meEverywhere": false,
                "unreadOnly": false,
                "bookmarksBar": true
            })
        );
        assert_eq!(
            serde_json::to_value(Mode::BookmarksBar).unwrap(),
            serde_json::json!("bookmarksBar")
        );
    }

    #[test]
    fn without_a_file_every_mode_is_off() {
        let dir = temp_config_dir();
        assert_eq!(ModesStore::open(dir.path()).get(), modes(false, false));
    }

    #[tokio::test]
    async fn each_mode_switches_on_and_off_independently() {
        let dir = temp_config_dir();
        let store = ModesStore::open(dir.path());

        assert_eq!(
            store.set(Mode::MeEverywhere, true).await.unwrap(),
            modes(true, false)
        );
        assert_eq!(
            store.set(Mode::UnreadOnly, true).await.unwrap(),
            modes(true, true)
        );
        assert_eq!(
            store.set(Mode::MeEverywhere, false).await.unwrap(),
            modes(false, true)
        );
        assert_eq!(
            store.set(Mode::UnreadOnly, false).await.unwrap(),
            modes(false, false)
        );
        assert_eq!(store.get(), modes(false, false));
    }

    #[tokio::test]
    async fn switched_modes_persist_across_reopen() {
        let dir = temp_config_dir();
        ModesStore::open(dir.path())
            .set(Mode::UnreadOnly, true)
            .await
            .unwrap();

        assert_eq!(ModesStore::open(dir.path()).get(), modes(false, true));
    }

    #[test]
    fn unknown_keys_are_ignored_and_known_ones_still_load() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join("modes.json"),
            r#"{"meEverywhere": true, "futureThing": [1, 2]}"#,
        )
        .unwrap();

        assert_eq!(ModesStore::open(dir.path()).get(), modes(true, false));
    }

    #[test]
    fn a_corrupt_file_falls_back_to_all_off_and_is_kept_as_a_backup() {
        let dir = temp_config_dir();
        fs::write(dir.path().join("modes.json"), "{ not json").unwrap();

        assert_eq!(ModesStore::open(dir.path()).get(), modes(false, false));
        let backups = fs::read_dir(dir.path())
            .unwrap()
            .filter(|e| {
                let name = e.as_ref().unwrap().file_name();
                name.to_string_lossy().ends_with(".corrupt")
            })
            .count();
        assert_eq!(backups, 1);
    }

    #[test]
    fn modes_and_mode_use_camel_case_on_the_wire() {
        assert_eq!(
            serde_json::to_value(modes(true, false)).unwrap(),
            serde_json::json!({
                "meEverywhere": true,
                "unreadOnly": false,
                "bookmarksBar": false
            })
        );
        assert_eq!(
            serde_json::to_value(Mode::UnreadOnly).unwrap(),
            serde_json::json!("unreadOnly")
        );
    }
}
