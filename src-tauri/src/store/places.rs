//! Where the owner left each view (`places.json`): scroll position, opened and closed groups, the selected
//! row and whether its peek was open. Restored when the view comes back, also after a restart. The Me chip and
//! the text filter aren't kept here (ADR 0038): the view saves its Me chip, the text is for the moment.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::json_store::JsonStore;
use crate::error::AppResult;

/// Older places are dropped beyond this many views.
const MAX_PLACES: usize = 200;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct ViewPlace {
    pub scroll_top: f64,
    /// Groups the owner opened (`true`) or closed (`false`); the others keep their default.
    pub toggled: BTreeMap<String, bool>,
    /// Parents whose subtasks the owner collapsed in Hierarchy (ADR 0037); the others show them.
    pub collapsed_parents: Vec<String>,
    pub selected_id: Option<String>,
    pub peek_open: bool,
    /// The Me chip as files from before ADR 0038 kept it; read once to seed `view.only_mine`, never written.
    #[serde(rename = "onlyMine", skip_serializing)]
    #[ts(skip)]
    pub legacy_only_mine: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
struct Places {
    /// By `"<listId>:<viewId>"`, with the order they were last saved in.
    views: BTreeMap<String, (u64, ViewPlace)>,
    next: u64,
}

pub struct PlaceStore {
    store: JsonStore<Places>,
}

fn key(list_id: i64, view_id: i64) -> String {
    format!("{list_id}:{view_id}")
}

impl PlaceStore {
    pub fn open(dir: &Path) -> Self {
        Self {
            store: JsonStore::open(dir.join("places.json")),
        }
    }

    pub fn get(&self, list_id: i64, view_id: i64) -> Option<ViewPlace> {
        self.store.read(|places| {
            places
                .views
                .get(&key(list_id, view_id))
                .map(|(_, place)| place.clone())
        })
    }

    /// The views whose place still holds the Me chip on, from before views saved it (ADR 0038).
    pub fn legacy_only_mine(&self) -> Vec<i64> {
        self.store.read(|places| {
            places
                .views
                .iter()
                .filter(|(_, (_, place))| place.legacy_only_mine)
                .filter_map(|(key, _)| key.split_once(':')?.1.parse().ok())
                .collect()
        })
    }

    /// Writes the file again without the old Me chips, so they seed the views only once.
    pub async fn forget_legacy_only_mine(&self) -> AppResult<()> {
        self.store
            .update_async(|places| {
                for (_, place) in places.views.values_mut() {
                    place.legacy_only_mine = false;
                }
            })
            .await
    }

    pub async fn set(&self, list_id: i64, view_id: i64, place: ViewPlace) -> AppResult<()> {
        self.store
            .update_async(move |places| {
                let order = places.next;
                places.next += 1;
                places.views.insert(key(list_id, view_id), (order, place));
                while places.views.len() > MAX_PLACES {
                    let oldest = places
                        .views
                        .iter()
                        .min_by_key(|(_, (order, _))| *order)
                        .map(|(key, _)| key.clone());
                    match oldest {
                        Some(key) => places.views.remove(&key),
                        None => break,
                    };
                }
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use std::fs;

    fn place(scroll_top: f64, selected: &str) -> ViewPlace {
        ViewPlace {
            scroll_top,
            toggled: BTreeMap::from([("review".to_string(), true), ("done".to_string(), false)]),
            selected_id: Some(selected.to_string()),
            peek_open: true,
            ..ViewPlace::default()
        }
    }

    #[test]
    fn a_view_never_saved_has_no_place() {
        let dir = temp_config_dir();

        assert_eq!(PlaceStore::open(dir.path()).get(1, 2), None);
    }

    #[tokio::test]
    async fn a_saved_place_is_returned_whole() {
        let dir = temp_config_dir();
        let store = PlaceStore::open(dir.path());

        store.set(1, 2, place(340.5, "10042")).await.unwrap();

        assert_eq!(store.get(1, 2), Some(place(340.5, "10042")));
    }

    #[tokio::test]
    async fn places_persist_across_reopen() {
        let dir = temp_config_dir();
        PlaceStore::open(dir.path())
            .set(1, 2, place(120.0, "10042"))
            .await
            .unwrap();

        assert_eq!(
            PlaceStore::open(dir.path()).get(1, 2),
            Some(place(120.0, "10042"))
        );
    }

    #[tokio::test]
    async fn separate_views_keep_separate_places() {
        let dir = temp_config_dir();
        let store = PlaceStore::open(dir.path());

        store.set(1, 2, place(10.0, "a")).await.unwrap();
        store.set(1, 3, place(20.0, "b")).await.unwrap();
        store.set(2, 2, place(30.0, "c")).await.unwrap();

        assert_eq!(store.get(1, 2), Some(place(10.0, "a")));
        assert_eq!(store.get(1, 3), Some(place(20.0, "b")));
        assert_eq!(store.get(2, 2), Some(place(30.0, "c")));
        assert_eq!(store.get(2, 3), None);
    }

    #[tokio::test]
    async fn saving_a_view_again_replaces_its_place() {
        let dir = temp_config_dir();
        let store = PlaceStore::open(dir.path());
        store.set(1, 2, place(10.0, "a")).await.unwrap();

        store.set(1, 2, ViewPlace::default()).await.unwrap();

        assert_eq!(store.get(1, 2), Some(ViewPlace::default()));
    }

    #[tokio::test]
    async fn the_oldest_saved_view_is_dropped_beyond_the_limit() {
        let dir = temp_config_dir();
        let store = PlaceStore::open(dir.path());

        for view in 0..=MAX_PLACES as i64 {
            store.set(1, view, place(view as f64, "x")).await.unwrap();
        }

        assert_eq!(store.get(1, 0), None);
        assert_eq!(store.get(1, 1), Some(place(1.0, "x")));
        let last = MAX_PLACES as i64;
        assert_eq!(store.get(1, last), Some(place(last as f64, "x")));
        assert_eq!(MAX_PLACES, 200);
    }

    #[tokio::test]
    async fn saving_an_old_view_again_makes_it_the_newest() {
        let dir = temp_config_dir();
        let store = PlaceStore::open(dir.path());
        for view in 0..MAX_PLACES as i64 {
            store.set(1, view, place(0.0, "x")).await.unwrap();
        }

        store.set(1, 0, place(99.0, "again")).await.unwrap();
        store
            .set(1, MAX_PLACES as i64, place(0.0, "x"))
            .await
            .unwrap();

        assert_eq!(store.get(1, 0), Some(place(99.0, "again")));
        assert_eq!(store.get(1, 1), None);
        assert_eq!(store.get(1, 2), Some(place(0.0, "x")));
    }

    #[test]
    fn a_place_serializes_with_camel_case_keys() {
        let value = serde_json::to_value(ViewPlace {
            legacy_only_mine: true,
            ..place(12.5, "10042")
        })
        .unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "scrollTop": 12.5,
                "toggled": { "done": false, "review": true },
                "collapsedParents": [],
                "selectedId": "10042",
                "peekOpen": true,
            })
        );
    }

    #[test]
    fn a_partial_place_fills_in_defaults() {
        let parsed: ViewPlace = serde_json::from_str(r#"{"scrollTop": 5}"#).unwrap();

        assert_eq!(
            parsed,
            ViewPlace {
                scroll_top: 5.0,
                ..ViewPlace::default()
            }
        );
    }

    const OLD_FILE: &str = r#"{"views": {
        "1:2": [0, {"scrollTop": 42.5, "toggled": {"done": false}, "selectedId": "10042", "peekOpen": true, "onlyMine": true, "text": "invoice"}],
        "1:3": [1, {"scrollTop": 8, "onlyMine": false, "text": "retro"}],
        "4:5": [2, {"scrollTop": 3, "onlyMine": true}]
    }, "next": 3}"#;

    fn write_old_file(dir: &Path) {
        fs::write(dir.join("places.json"), OLD_FILE).unwrap();
    }

    #[test]
    fn a_file_with_the_old_me_chip_and_text_keys_still_loads_its_other_fields() {
        let dir = temp_config_dir();
        write_old_file(dir.path());

        let loaded = PlaceStore::open(dir.path()).get(1, 2).unwrap();

        assert_eq!(
            loaded,
            ViewPlace {
                scroll_top: 42.5,
                toggled: BTreeMap::from([("done".to_string(), false)]),
                selected_id: Some("10042".to_string()),
                peek_open: true,
                legacy_only_mine: true,
                collapsed_parents: Vec::new(),
            }
        );
    }

    #[test]
    fn the_legacy_me_chip_lists_the_views_that_had_it_on() {
        let dir = temp_config_dir();
        write_old_file(dir.path());

        let mut ids = PlaceStore::open(dir.path()).legacy_only_mine();
        ids.sort();

        assert_eq!(ids, [2, 5]);
    }

    #[test]
    fn a_file_without_the_old_keys_has_no_legacy_me_chip() {
        let dir = temp_config_dir();
        fs::write(
            dir.path().join("places.json"),
            r#"{"views": {"1:2": [0, {"scrollTop": 1}]}, "next": 1}"#,
        )
        .unwrap();

        assert_eq!(
            PlaceStore::open(dir.path()).legacy_only_mine(),
            Vec::<i64>::new()
        );
    }

    #[tokio::test]
    async fn forgetting_the_legacy_me_chip_removes_it_from_the_file_and_keeps_the_rest() {
        let dir = temp_config_dir();
        write_old_file(dir.path());
        let store = PlaceStore::open(dir.path());

        store.forget_legacy_only_mine().await.unwrap();

        assert_eq!(store.legacy_only_mine(), Vec::<i64>::new());
        let reopened = PlaceStore::open(dir.path());
        assert_eq!(reopened.legacy_only_mine(), Vec::<i64>::new());
        assert_eq!(
            reopened.get(1, 2),
            Some(ViewPlace {
                scroll_top: 42.5,
                toggled: BTreeMap::from([("done".to_string(), false)]),
                selected_id: Some("10042".to_string()),
                peek_open: true,
                ..ViewPlace::default()
            })
        );
        let written = fs::read_to_string(dir.path().join("places.json")).unwrap();
        assert!(!written.contains("onlyMine"), "{written}");
        assert!(!written.contains("text"), "{written}");
    }

    #[tokio::test]
    async fn a_saved_place_never_writes_the_me_chip_or_text_keys() {
        let dir = temp_config_dir();
        let store = PlaceStore::open(dir.path());

        store
            .set(
                1,
                2,
                ViewPlace {
                    legacy_only_mine: true,
                    ..place(7.0, "10042")
                },
            )
            .await
            .unwrap();

        let written = fs::read_to_string(dir.path().join("places.json")).unwrap();
        assert!(!written.contains("onlyMine"), "{written}");
        assert!(!written.contains("\"text\""), "{written}");
        assert_eq!(
            PlaceStore::open(dir.path()).legacy_only_mine(),
            Vec::<i64>::new()
        );
    }

    #[tokio::test]
    async fn collapsed_parents_persist_across_reopen() {
        let dir = temp_config_dir();
        let saved = ViewPlace {
            collapsed_parents: vec!["10042".to_string(), "10047".to_string()],
            ..place(7.0, "10042")
        };
        PlaceStore::open(dir.path())
            .set(1, 2, saved.clone())
            .await
            .unwrap();

        assert_eq!(PlaceStore::open(dir.path()).get(1, 2), Some(saved));
    }

    #[test]
    fn collapsed_parents_serialize_in_camel_case_and_default_to_empty() {
        let value = serde_json::to_value(ViewPlace {
            collapsed_parents: vec!["10042".to_string()],
            ..ViewPlace::default()
        })
        .unwrap();
        let parsed: ViewPlace = serde_json::from_str(r#"{"scrollTop": 5}"#).unwrap();

        assert_eq!(value["collapsedParents"], serde_json::json!(["10042"]));
        assert!(parsed.collapsed_parents.is_empty());
    }

    #[test]
    fn a_corrupt_file_loads_empty() {
        let dir = temp_config_dir();
        fs::write(dir.path().join("places.json"), "{ not json").unwrap();

        assert_eq!(PlaceStore::open(dir.path()).get(1, 2), None);
    }

    #[test]
    fn a_file_of_the_wrong_shape_loads_empty() {
        let dir = temp_config_dir();
        fs::write(dir.path().join("places.json"), r#"{"views": [1, 2, 3]}"#).unwrap();

        assert_eq!(PlaceStore::open(dir.path()).get(1, 2), None);
    }

    #[tokio::test]
    async fn a_store_that_loaded_empty_from_a_corrupt_file_can_save_again() {
        let dir = temp_config_dir();
        fs::write(dir.path().join("places.json"), "garbage").unwrap();
        let store = PlaceStore::open(dir.path());

        store.set(1, 2, place(1.0, "a")).await.unwrap();

        assert_eq!(
            PlaceStore::open(dir.path()).get(1, 2),
            Some(place(1.0, "a"))
        );
    }
}
