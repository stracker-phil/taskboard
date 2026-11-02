//! Saved searches: the filter lists that only search for text, as the palette's ⌘S saves them.

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppResult;
use crate::filter::{Field, Filter, Value};
use crate::lists::{Kind, all_nodes};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SavedSearch {
    #[ts(type = "number")]
    pub list_id: i64,
    pub name: String,
    /// What it searches for.
    pub text: String,
    /// The Space it sits in; none at the top of the sidebar.
    pub space: Option<String>,
}

/// Every filter list whose one condition searches for text, in sidebar order.
pub fn saved_searches(conn: &Connection) -> AppResult<Vec<SavedSearch>> {
    let nodes = all_nodes(conn)?;
    Ok(nodes
        .iter()
        .filter(|node| node.kind == Kind::List)
        .filter_map(|node| {
            let filter = Filter::parse(node.filter.as_deref()).ok()?;
            let [condition] = filter.conditions.as_slice() else {
                return None;
            };
            let Some(Value::One(text)) = (condition.field == Field::Text)
                .then_some(condition.value.as_ref())
                .flatten()
            else {
                return None;
            };
            let space = nodes
                .iter()
                .find(|parent| Some(parent.id) == node.parent_id && parent.kind == Kind::Space)
                .map(|parent| parent.name.clone());
            Some(SavedSearch {
                list_id: node.id,
                name: node.name.clone(),
                text: text.clone(),
                space,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::filter::{Condition, Op};
    use crate::lists::{Viewer, list_create};
    use crate::test_support::temp_config_dir;
    use chrono::Utc;
    use tempfile::TempDir;

    struct Site {
        _dir: TempDir,
        db: Db,
    }

    impl Site {
        fn new() -> Self {
            let dir = temp_config_dir();
            let db = Db::open(dir.path()).unwrap();
            Self { _dir: dir, db }
        }

        async fn node_id(&self, sql: &'static str) -> i64 {
            self.db
                .read(move |conn| Ok(conn.query_row(sql, [], |row| row.get(0))?))
                .await
                .unwrap()
        }

        async fn space(&self, name: &'static str) -> i64 {
            self.db
                .write(move |tx, _| {
                    tx.execute(
                        "INSERT INTO filter_list (kind, project_id, name) VALUES ('space', ?1, ?1)",
                        [name],
                    )?;
                    Ok(tx.last_insert_rowid())
                })
                .await
                .unwrap()
        }

        async fn everything(&self) -> i64 {
            self.node_id("SELECT id FROM filter_list WHERE kind = 'everything'")
                .await
        }

        async fn list(&self, parent: i64, name: &str, filter: Option<Filter>) -> i64 {
            let viewer = Viewer {
                me: None,
                time_zone: None,
                now: Utc::now(),
            };
            list_create(&self.db, viewer, parent, name.into(), filter)
                .await
                .unwrap()
        }

        async fn saved(&self) -> Vec<SavedSearch> {
            self.db.read(saved_searches).await.unwrap()
        }
    }

    fn text_contains(text: &str) -> Filter {
        Filter {
            conditions: vec![Condition::new(
                Field::Text,
                Op::Contains,
                Some(Value::One(text.into())),
            )],
        }
    }

    #[tokio::test]
    async fn a_text_only_list_is_a_saved_search_with_its_space() {
        let site = Site::new();
        let space = site.space("Website relaunch").await;
        let list = site
            .list(space, "Checkout talk", Some(text_contains("checkout")))
            .await;

        assert_eq!(
            site.saved().await,
            [SavedSearch {
                list_id: list,
                name: "Checkout talk".into(),
                text: "checkout".into(),
                space: Some("Website relaunch".into()),
            }]
        );
    }

    #[tokio::test]
    async fn lists_that_do_more_than_search_for_text_are_left_out() {
        let site = Site::new();
        let space = site.space("Website relaunch").await;
        let epics = Filter {
            conditions: vec![Condition::is(Field::Type, "Epic")],
        };
        let mut text_and_type = text_contains("checkout");
        text_and_type
            .conditions
            .push(Condition::is(Field::Type, "Epic"));
        site.list(space, "Epics only", Some(epics)).await;
        site.list(space, "Text and type", Some(text_and_type)).await;
        site.list(space, "No filter", None).await;
        let kept = site
            .list(space, "Checkout talk", Some(text_contains("checkout")))
            .await;

        let saved = site.saved().await;

        assert_eq!(saved.iter().map(|s| s.list_id).collect::<Vec<_>>(), [kept]);
    }

    #[tokio::test]
    async fn a_text_list_outside_any_space_has_no_space() {
        let site = Site::new();
        let everything = site.everything().await;
        let list = site
            .list(everything, "Invoices", Some(text_contains("invoice")))
            .await;

        assert_eq!(
            site.saved().await,
            [SavedSearch {
                list_id: list,
                name: "Invoices".into(),
                text: "invoice".into(),
                space: None,
            }]
        );
    }

    #[tokio::test]
    async fn saved_searches_come_in_sidebar_order() {
        let site = Site::new();
        let space = site.space("Website relaunch").await;
        for name in ["Zebra", "Apple", "Mango"] {
            site.list(space, name, Some(text_contains("checkout")))
                .await;
        }

        let names: Vec<String> = site.saved().await.into_iter().map(|s| s.name).collect();

        assert_eq!(names, ["Zebra", "Apple", "Mango"]);
    }

    #[test]
    fn a_saved_search_serializes_with_camel_case_keys() {
        let value = serde_json::to_value(SavedSearch {
            list_id: 7,
            name: "Checkout talk".into(),
            text: "checkout".into(),
            space: None,
        })
        .unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "listId": 7,
                "name": "Checkout talk",
                "text": "checkout",
                "space": null
            })
        );
    }
}
