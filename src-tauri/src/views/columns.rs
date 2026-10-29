//! The columns a view can show: the standard fields, then the site's custom fields by their Jira ids.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::db::Db;
use crate::error::AppResult;
use crate::filter::RELEVANT_FIELD;
use crate::space_config::hidden_columns;

/// Standard column ids with their names, in the Columns popover's order.
const STANDARD: [(&str, &str); 9] = [
    ("status", "Status"),
    ("statusType", "Status type"),
    ("assignee", "Assignee"),
    ("epic", "Epic"),
    ("release", "Release"),
    ("priority", "Priority"),
    ("labels", "Labels"),
    ("sprint", "Sprint"),
    ("due", "Due"),
];

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// A standard id or a Jira field id (`customfield_10071`).
    pub id: String,
    pub name: String,
    pub custom: bool,
}

/// A custom field a column can show, with Jira's schema type (`user`, `number`, `option`, …).
#[derive(Clone, Debug, PartialEq)]
pub struct CustomColumn {
    pub id: String,
    pub kind: String,
}

/// Custom fields with a value a cell can show. Jira's `any` fields (rank, epic color) are internal, and the
/// sprint field is the standard Sprint column.
pub const SHOWABLE: &str =
    "id LIKE 'customfield\\_%' ESCAPE '\\' AND COALESCE(type, '') <> 'any' AND name <> 'Sprint'";

/// The columns a level's views can show: the standard ones and the custom fields its Space doesn't hide.
pub fn catalog(conn: &Connection, list_id: i64) -> AppResult<Vec<Column>> {
    let hidden = hidden_columns(conn, list_id)?;
    let mut columns: Vec<Column> = STANDARD
        .iter()
        .map(|(id, name)| Column {
            id: id.to_string(),
            name: name.to_string(),
            custom: false,
        })
        .collect();
    let mut query = conn.prepare(&format!(
        "SELECT id, name FROM cache.field WHERE {SHOWABLE} AND {RELEVANT_FIELD}
         ORDER BY name COLLATE NOCASE, id"
    ))?;
    let custom = query.query_map([], |row| {
        Ok(Column {
            id: row.get(0)?,
            name: row.get(1)?,
            custom: true,
        })
    })?;
    columns.extend(
        custom
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|column| !hidden.contains(&column.id)),
    );
    Ok(columns)
}

/// The custom fields among `ids`, in that order; unknown ids are left out.
pub fn custom_columns(conn: &Connection, ids: &[String]) -> AppResult<Vec<CustomColumn>> {
    let mut query = conn.prepare(&format!(
        "SELECT COALESCE(type, '') FROM cache.field WHERE id = ?1 AND {SHOWABLE}"
    ))?;
    let mut found = Vec::new();
    for id in ids {
        if let Some(kind) = query.query_row([id], |row| row.get(0)).optional()? {
            found.push(CustomColumn {
                id: id.clone(),
                kind,
            });
        }
    }
    Ok(found)
}

pub async fn columns_catalogue(db: &Db, list_id: i64) -> AppResult<Vec<Column>> {
    db.read(move |conn| catalog(conn, list_id)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::space_config::{FieldVisibility, space_set_field};

    const ZONE: &str = "customfield_10001";
    const BETA: &str = "customfield_10003";

    async fn lab() -> (tempfile::TempDir, Db) {
        let dir = crate::test_support::temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO cache.space (id, key, name, raw) VALUES
                   ('10001', 'NW', 'Website relaunch', '{}'), ('10002', 'NWS', 'Northwind support', '{}');
                 INSERT INTO cache.field (id, name, type, used) VALUES
                   ('customfield_10001', 'Zone', 'string', 1),
                   ('customfield_10003', 'Beta', 'string', 1),
                   ('customfield_10019', 'Rank', 'any', 1)",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        for id in ["10001", "10002"] {
            crate::connection::track_project(&db, id.into(), true)
                .await
                .unwrap();
        }
        (dir, db)
    }

    async fn level(db: &Db, clause: &str) -> i64 {
        let sql = format!("SELECT id FROM filter_list WHERE {clause} ORDER BY id LIMIT 1");
        db.read(move |conn| Ok(conn.query_row(&sql, [], |r| r.get(0))?))
            .await
            .unwrap()
    }

    async fn custom_ids(db: &Db, list_id: i64) -> Vec<String> {
        columns_catalogue(db, list_id)
            .await
            .unwrap()
            .into_iter()
            .filter(|c| c.custom)
            .map(|c| c.id)
            .collect()
    }

    #[tokio::test]
    async fn the_standard_columns_come_first_and_internal_fields_are_left_out() {
        let (_dir, db) = lab().await;
        let everything = level(&db, "kind = 'everything'").await;

        let columns = columns_catalogue(&db, everything).await.unwrap();

        let ids: Vec<&str> = columns.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "status",
                "statusType",
                "assignee",
                "epic",
                "release",
                "priority",
                "labels",
                "sprint",
                "due",
                BETA,
                ZONE
            ]
        );
        let status_type = columns.iter().find(|c| c.id == "statusType").unwrap();
        assert_eq!(status_type.name, "Status type");
        assert!(!status_type.custom);
    }

    #[tokio::test]
    async fn a_space_leaves_out_the_custom_fields_it_hides_from_lists() {
        let (_dir, db) = lab().await;
        let space_a = level(&db, "kind = 'space' AND project_id = '10001'").await;
        let list_in_a = level(
            &db,
            "kind = 'list' AND parent_id IN
               (SELECT id FROM filter_list WHERE kind = 'space' AND project_id = '10001')",
        )
        .await;
        let space_b = level(&db, "kind = 'space' AND project_id = '10002'").await;
        space_set_field(
            &db,
            "10001".into(),
            ZONE.into(),
            FieldVisibility::HiddenInList,
        )
        .await
        .unwrap();
        space_set_field(
            &db,
            "10001".into(),
            BETA.into(),
            FieldVisibility::HiddenInDetail,
        )
        .await
        .unwrap();

        assert_eq!(custom_ids(&db, space_a).await, [BETA]);
        assert_eq!(custom_ids(&db, list_in_a).await, [BETA]);
        assert_eq!(custom_ids(&db, space_b).await, [BETA, ZONE]);
    }

    #[tokio::test]
    async fn outside_the_spaces_a_field_stays_while_one_space_shows_it() {
        let (_dir, db) = lab().await;
        let everything = level(&db, "kind = 'everything'").await;
        space_set_field(&db, "10001".into(), ZONE.into(), FieldVisibility::Hidden)
            .await
            .unwrap();

        assert_eq!(custom_ids(&db, everything).await, [BETA, ZONE]);

        space_set_field(
            &db,
            "10002".into(),
            ZONE.into(),
            FieldVisibility::HiddenInList,
        )
        .await
        .unwrap();

        assert_eq!(custom_ids(&db, everything).await, [BETA]);
    }
}
