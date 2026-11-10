//! Which custom fields show at all, app-wide (`field_choice`, ADR 0056): in the list (a column and filter
//! choice), in the Issue view, or nowhere. A Space only narrows this (`space_config`).

pub mod commands;

use std::collections::HashSet;

use rusqlite::{Connection, params};
use serde::Serialize;
use ts_rs::TS;

use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::space_config::Place;
use crate::views::columns::SHOWABLE;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct FieldChoiceRow {
    pub id: String,
    pub name: String,
    /// Jira's schema type: `user`, `option`, `string`, `number`, `date`, …
    pub kind: String,
    pub in_list: bool,
    pub in_detail: bool,
}

/// Every custom field of the site a cell can show, by name.
pub fn choice_rows(conn: &Connection) -> AppResult<Vec<FieldChoiceRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT id, name, COALESCE(type, ''), COALESCE(c.in_list, 0), COALESCE(c.in_detail, 0)
         FROM cache.field LEFT JOIN field_choice c ON c.field_id = id
         WHERE {SHOWABLE}
         ORDER BY name COLLATE NOCASE, id"
    ))?;
    let rows = query.query_map([], |row| {
        Ok(FieldChoiceRow {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            in_list: row.get(3)?,
            in_detail: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub async fn field_choices(db: &Db) -> AppResult<Vec<FieldChoiceRow>> {
    db.read(choice_rows).await
}

/// Unticking both places deletes the row, the default: the field shows nowhere.
pub async fn field_choice_set(
    db: &Db,
    field_id: String,
    in_list: bool,
    in_detail: bool,
) -> AppResult<Vec<FieldChoiceRow>> {
    db.write(move |tx, changes| {
        ensure_showable(tx, &field_id)?;
        store_choice(tx, &field_id, in_list, in_detail)?;
        changes.local_changed(LocalPart::Views);
        changes.local_changed(LocalPart::Spaces);
        choice_rows(tx)
    })
    .await
}

fn ensure_showable(conn: &Connection, field_id: &str) -> AppResult<()> {
    let known: bool = conn.query_row(
        &format!("SELECT EXISTS (SELECT 1 FROM cache.field WHERE id = ?1 AND {SHOWABLE})"),
        [field_id],
        |row| row.get(0),
    )?;
    if known {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "Field {field_id} doesn't exist"
        )))
    }
}

fn store_choice(
    conn: &Connection,
    field_id: &str,
    in_list: bool,
    in_detail: bool,
) -> AppResult<()> {
    if in_list || in_detail {
        conn.execute(
            "INSERT INTO field_choice (field_id, in_list, in_detail) VALUES (?1, ?2, ?3)
             ON CONFLICT (field_id) DO UPDATE SET in_list = excluded.in_list, in_detail = excluded.in_detail",
            params![field_id, in_list, in_detail],
        )?;
    } else {
        conn.execute("DELETE FROM field_choice WHERE field_id = ?1", [field_id])?;
    }
    Ok(())
}

/// A filter or a column on a field puts it in the list, so the next sync downloads it.
pub fn show_in_list(conn: &Connection, field_id: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO field_choice (field_id, in_list, in_detail) VALUES (?1, 1, 0)
         ON CONFLICT (field_id) DO UPDATE SET in_list = 1",
        [field_id],
    )?;
    Ok(())
}

/// Also Jira's internal fields, which never show.
pub fn not_chosen(conn: &Connection, place: Place) -> AppResult<HashSet<String>> {
    let mut query = conn.prepare(&format!(
        "SELECT id FROM cache.field WHERE id LIKE 'customfield\\_%' ESCAPE '\\'
           AND id NOT IN (SELECT field_id FROM field_choice WHERE {} = 1)",
        column(place)
    ))?;
    let ids = query.query_map([], |row| row.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

fn column(place: Place) -> &'static str {
    match place {
        Place::List => "in_list",
        Place::Detail => "in_detail",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::test_support::temp_config_dir;

    const ZONE: &str = "customfield_10001";
    const ALPHA: &str = "customfield_10002";
    const NOTES: &str = "customfield_10003";

    async fn lab() -> (tempfile::TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO cache.field (id, name, type) VALUES
                   ('summary', 'Summary', 'string'),
                   ('customfield_10001', 'Zone', 'string'),
                   ('customfield_10002', 'alpha', 'user'),
                   ('customfield_10003', 'Notes', NULL),
                   ('customfield_10019', 'Rank', 'any'),
                   ('customfield_10020', 'Sprint', 'array')",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        (dir, db)
    }

    fn row(id: &str, name: &str, kind: &str, in_list: bool, in_detail: bool) -> FieldChoiceRow {
        FieldChoiceRow {
            id: id.into(),
            name: name.into(),
            kind: kind.into(),
            in_list,
            in_detail,
        }
    }

    async fn stored(db: &Db) -> Vec<(String, bool, bool)> {
        db.read(|conn| {
            let mut query = conn.prepare(
                "SELECT field_id, in_list, in_detail FROM field_choice ORDER BY field_id",
            )?;
            let rows = query.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn every_showable_custom_field_is_listed_by_name_and_unticked_by_default() {
        let (_dir, db) = lab().await;

        let rows = field_choices(&db).await.unwrap();

        assert_eq!(
            rows,
            [
                row(ALPHA, "alpha", "user", false, false),
                row(NOTES, "Notes", "", false, false),
                row(ZONE, "Zone", "string", false, false),
            ]
        );
    }

    #[tokio::test]
    async fn a_choice_is_stored_changed_and_cleared_when_both_boxes_are_unticked() {
        let (_dir, db) = lab().await;

        let rows = field_choice_set(&db, ZONE.into(), true, false)
            .await
            .unwrap();

        assert_eq!(rows[2], row(ZONE, "Zone", "string", true, false));
        assert_eq!(stored(&db).await, [(ZONE.to_string(), true, false)]);

        let rows = field_choice_set(&db, ZONE.into(), false, true)
            .await
            .unwrap();

        assert_eq!(rows[2], row(ZONE, "Zone", "string", false, true));
        assert_eq!(stored(&db).await, [(ZONE.to_string(), false, true)]);

        let rows = field_choice_set(&db, ZONE.into(), false, false)
            .await
            .unwrap();

        assert_eq!(rows[2], row(ZONE, "Zone", "string", false, false));
        assert_eq!(stored(&db).await, []);
    }

    #[tokio::test]
    async fn choices_of_different_fields_are_independent() {
        let (_dir, db) = lab().await;
        field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();

        let rows = field_choice_set(&db, ALPHA.into(), true, false)
            .await
            .unwrap();

        assert_eq!(
            rows,
            [
                row(ALPHA, "alpha", "user", true, false),
                row(NOTES, "Notes", "", false, false),
                row(ZONE, "Zone", "string", true, true),
            ]
        );
    }

    #[tokio::test]
    async fn an_unknown_or_unshowable_field_is_not_found_and_stores_nothing() {
        let (_dir, db) = lab().await;

        for id in [
            "customfield_99999",
            "customfield_10019",
            "customfield_10020",
            "summary",
        ] {
            let err = field_choice_set(&db, id.into(), true, true)
                .await
                .unwrap_err();

            assert_eq!(err.kind(), ErrorKind::NotFound, "{id}");
        }
        assert_eq!(stored(&db).await, []);
    }

    #[tokio::test]
    async fn showing_a_field_in_the_list_ticks_the_list_and_keeps_the_detail_box() {
        let (_dir, db) = lab().await;
        field_choice_set(&db, ZONE.into(), false, true)
            .await
            .unwrap();

        db.write(|tx, _| {
            show_in_list(tx, ZONE)?;
            show_in_list(tx, ALPHA)
        })
        .await
        .unwrap();

        assert_eq!(
            stored(&db).await,
            [
                (ZONE.to_string(), true, true),
                (ALPHA.to_string(), true, false),
            ]
        );
    }

    #[tokio::test]
    async fn the_fields_not_chosen_depend_on_the_place() {
        let (_dir, db) = lab().await;
        field_choice_set(&db, ZONE.into(), true, false)
            .await
            .unwrap();
        field_choice_set(&db, ALPHA.into(), false, true)
            .await
            .unwrap();

        let (not_in_list, not_in_detail) = db
            .read(|conn| {
                Ok((
                    not_chosen(conn, Place::List)?,
                    not_chosen(conn, Place::Detail)?,
                ))
            })
            .await
            .unwrap();

        let ids =
            |items: &[&str]| -> HashSet<String> { items.iter().map(|id| id.to_string()).collect() };
        assert_eq!(
            not_in_list,
            ids(&[ALPHA, NOTES, "customfield_10019", "customfield_10020"])
        );
        assert_eq!(
            not_in_detail,
            ids(&[ZONE, NOTES, "customfield_10019", "customfield_10020"])
        );
    }

    #[test]
    fn a_choice_row_serializes_for_the_page_in_camel_case() {
        assert_eq!(
            serde_json::to_value(row(ZONE, "Zone", "string", true, false)).unwrap(),
            serde_json::json!({
                "id": ZONE,
                "name": "Zone",
                "kind": "string",
                "inList": true,
                "inDetail": false,
            })
        );
    }
}
