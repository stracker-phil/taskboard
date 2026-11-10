//! Which custom fields show at all, app-wide (`field_choice`, ADR 0056): in the list (a column and filter
//! choice), in the Issue view, or nowhere. A Space only narrows this (`space_config`).

pub mod commands;
mod forget;

use std::collections::HashSet;

use rusqlite::{Connection, params};
use serde::Serialize;
use ts_rs::TS;

use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::space_config::Place;
use crate::sync::store::fetched_fields;
use crate::views::columns::SHOWABLE;
use forget::forget_values;

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

/// The rows after a choice, and whether the field is downloaded now and wasn't before: then only a full sync
/// fetches its values for every issue.
pub struct ChoiceSaved {
    pub rows: Vec<FieldChoiceRow>,
    pub newly_fetched: bool,
}

/// Unticking both places deletes the row, the default: the field shows nowhere. A field no longer fetched loses
/// its cached values.
pub async fn field_choice_set(
    db: &Db,
    field_id: String,
    in_list: bool,
    in_detail: bool,
) -> AppResult<ChoiceSaved> {
    db.write(move |tx, changes| {
        ensure_showable(tx, &field_id)?;
        let was_fetched = fetched_fields(tx)?.contains(&field_id);
        store_choice(tx, &field_id, in_list, in_detail)?;
        let is_fetched = fetched_fields(tx)?.contains(&field_id);
        if was_fetched && !is_fetched {
            forget_values(tx, changes, &field_id)?;
        }
        changes.local_changed(LocalPart::Views);
        changes.local_changed(LocalPart::Spaces);
        Ok(ChoiceSaved {
            rows: choice_rows(tx)?,
            newly_fetched: !was_fetched && is_fetched,
        })
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

        let saved = field_choice_set(&db, ZONE.into(), true, false)
            .await
            .unwrap();

        assert_eq!(saved.rows[2], row(ZONE, "Zone", "string", true, false));
        assert_eq!(stored(&db).await, [(ZONE.to_string(), true, false)]);

        let saved = field_choice_set(&db, ZONE.into(), false, true)
            .await
            .unwrap();

        assert_eq!(saved.rows[2], row(ZONE, "Zone", "string", false, true));
        assert_eq!(stored(&db).await, [(ZONE.to_string(), false, true)]);

        let saved = field_choice_set(&db, ZONE.into(), false, false)
            .await
            .unwrap();

        assert_eq!(saved.rows[2], row(ZONE, "Zone", "string", false, false));
        assert_eq!(stored(&db).await, []);
    }

    #[tokio::test]
    async fn the_first_box_on_a_never_fetched_field_asks_for_a_full_sync() {
        let (_dir, db) = lab().await;

        let saved = field_choice_set(&db, ZONE.into(), false, true)
            .await
            .unwrap();

        assert!(saved.newly_fetched);
    }

    #[tokio::test]
    async fn a_further_box_on_an_already_fetched_field_does_not() {
        let (_dir, db) = lab().await;
        field_choice_set(&db, ZONE.into(), true, false)
            .await
            .unwrap();

        let more = field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();
        let moved = field_choice_set(&db, ZONE.into(), false, true)
            .await
            .unwrap();

        assert!(!more.newly_fetched);
        assert!(!moved.newly_fetched);
    }

    #[tokio::test]
    async fn a_field_a_view_column_or_a_filter_already_uses_is_not_newly_fetched() {
        let (_dir, db) = lab().await;
        db.write(|tx, _| {
            tx.execute_batch(
                r#"INSERT INTO filter_list (id, kind, name, filter) VALUES
                     (900, 'list', 'Open work', '{"conditions":[{"field":"customfield_10002","op":"is","value":"x"}]}');
                   INSERT INTO view (list_id, name, layout, columns, group_by, sort_by, sort_order) VALUES
                     (900, 'Table', 'table', '["status","customfield_10001"]', 'none', 'updated', 0)"#,
            )?;
            Ok(())
        })
        .await
        .unwrap();

        let by_column = field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();
        let by_filter = field_choice_set(&db, ALPHA.into(), true, true)
            .await
            .unwrap();
        let unused = field_choice_set(&db, NOTES.into(), true, true)
            .await
            .unwrap();

        assert!(!by_column.newly_fetched);
        assert!(!by_filter.newly_fetched);
        assert!(unused.newly_fetched);
    }

    #[tokio::test]
    async fn unticking_both_boxes_never_asks_for_a_sync() {
        let (_dir, db) = lab().await;

        let unchosen = field_choice_set(&db, ZONE.into(), false, false)
            .await
            .unwrap();
        field_choice_set(&db, ALPHA.into(), true, true)
            .await
            .unwrap();
        let cleared = field_choice_set(&db, ALPHA.into(), false, false)
            .await
            .unwrap();

        assert!(!unchosen.newly_fetched);
        assert!(!cleared.newly_fetched);
    }

    #[tokio::test]
    async fn choices_of_different_fields_are_independent() {
        let (_dir, db) = lab().await;
        field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();

        let saved = field_choice_set(&db, ALPHA.into(), true, false)
            .await
            .unwrap();

        assert_eq!(
            saved.rows,
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
                .err()
                .unwrap();

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

    async fn lab_with_issues() -> (tempfile::TempDir, Db) {
        let (dir, db) = lab().await;
        db.write(|tx, _| {
            tx.execute_batch(
                r#"INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Website relaunch', '{}');
                   INSERT INTO cache.status (id, name, category) VALUES ('s1', 'To Do', 'new');
                   INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
                     custom, created_at, updated_at, synced_at) VALUES
                     ('1', 'NW-1', '10001', 'One', 'Task', 's1', 'new',
                      '{"customfield_10001":"Berlin","customfield_10002":{"accountId":"acc-1"}}',
                      '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z'),
                     ('2', 'NW-2', '10001', 'Two', 'Task', 's1', 'new',
                      '{"customfield_10001":"Paris"}',
                      '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z'),
                     ('3', 'NW-3', '10001', 'Three', 'Task', 's1', 'new',
                      '{"customfield_10002":{"accountId":"acc-2"}}',
                      '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z');
                   INSERT INTO cache.issue_body (issue_id, raw) VALUES
                     ('1', '{"id":"1","renderedFields":{"customfield_10001":"<p>Berlin</p>","customfield_10002":"<p>Ann</p>"}}'),
                     ('2', '{"id":"2","renderedFields":{"customfield_10001":"<p>Paris</p>"}}'),
                     ('3', '{"id":"3","renderedFields":{"customfield_10002":"<p>Bo</p>"}}')"#,
            )?;
            Ok(())
        })
        .await
        .unwrap();
        (dir, db)
    }

    async fn custom_of(db: &Db, id: &'static str) -> Option<serde_json::Value> {
        let text: Option<String> = db
            .read(move |conn| {
                Ok(
                    conn.query_row("SELECT custom FROM cache.issue WHERE id = ?1", [id], |r| {
                        r.get(0)
                    })?,
                )
            })
            .await
            .unwrap();
        text.map(|t| serde_json::from_str(&t).unwrap())
    }

    async fn raw_of(db: &Db, id: &'static str) -> serde_json::Value {
        let text: String = db
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT raw FROM cache.issue_body WHERE issue_id = ?1",
                    [id],
                    |r| r.get(0),
                )?)
            })
            .await
            .unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn announced_issues(db: &Db) -> std::sync::Arc<std::sync::Mutex<Vec<crate::db::Changes>>> {
        let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let writer = std::sync::Arc::clone(&sink);
        db.set_notifier(Box::new(move |changes| {
            writer.lock().unwrap().push(changes.clone());
        }));
        sink
    }

    #[tokio::test]
    async fn unticking_a_fetched_field_strips_its_values_and_keeps_the_other_fields() {
        let (_dir, db) = lab_with_issues().await;
        field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();

        field_choice_set(&db, ZONE.into(), false, false)
            .await
            .unwrap();

        assert_eq!(
            custom_of(&db, "1").await,
            Some(serde_json::json!({ ALPHA: { "accountId": "acc-1" } }))
        );
        assert_eq!(
            custom_of(&db, "3").await,
            Some(serde_json::json!({ ALPHA: { "accountId": "acc-2" } }))
        );
        assert_eq!(
            raw_of(&db, "1").await,
            serde_json::json!({ "id": "1", "renderedFields": { ALPHA: "<p>Ann</p>" } })
        );
        assert_eq!(
            raw_of(&db, "3").await,
            serde_json::json!({ "id": "3", "renderedFields": { ALPHA: "<p>Bo</p>" } })
        );
    }

    #[tokio::test]
    async fn an_issue_whose_only_custom_value_was_the_field_ends_with_no_custom_values() {
        let (_dir, db) = lab_with_issues().await;
        field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();

        field_choice_set(&db, ZONE.into(), false, false)
            .await
            .unwrap();

        assert_eq!(custom_of(&db, "2").await, None);
        assert_eq!(
            raw_of(&db, "2").await,
            serde_json::json!({ "id": "2", "renderedFields": {} })
        );
    }

    #[tokio::test]
    async fn the_write_announces_exactly_the_issues_that_lost_a_value() {
        let (_dir, db) = lab_with_issues().await;
        field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();
        let sink = announced_issues(&db);

        field_choice_set(&db, ZONE.into(), false, false)
            .await
            .unwrap();

        let announced = sink.lock().unwrap();
        assert_eq!(announced.len(), 1);
        assert_eq!(
            announced[0].issues,
            ["1", "2"].into_iter().map(String::from).collect()
        );
    }

    #[tokio::test]
    async fn a_field_a_filter_or_a_view_column_still_uses_keeps_its_values() {
        for (name, setup) in [
            (
                "filter",
                r#"INSERT INTO filter_list (id, kind, name, filter) VALUES
                     (900, 'list', 'Open work', '{"conditions":[{"field":"customfield_10001","op":"is","value":"x"}]}')"#,
            ),
            (
                "view column",
                r#"INSERT INTO filter_list (id, kind, name) VALUES (900, 'list', 'Open work');
                   INSERT INTO view (list_id, name, layout, columns, group_by, sort_by, sort_order) VALUES
                     (900, 'Table', 'table', '["status","customfield_10001"]', 'none', 'updated', 0)"#,
            ),
        ] {
            let (_dir, db) = lab_with_issues().await;
            db.write(move |tx, _| {
                tx.execute_batch(setup)?;
                Ok(())
            })
            .await
            .unwrap();
            field_choice_set(&db, ZONE.into(), true, true)
                .await
                .unwrap();
            let sink = announced_issues(&db);

            field_choice_set(&db, ZONE.into(), false, false)
                .await
                .unwrap();

            assert_eq!(
                custom_of(&db, "2").await,
                Some(serde_json::json!({ ZONE: "Paris" })),
                "{name}"
            );
            assert_eq!(
                raw_of(&db, "2").await,
                serde_json::json!({ "id": "2", "renderedFields": { ZONE: "<p>Paris</p>" } }),
                "{name}"
            );
            assert!(sink.lock().unwrap()[0].issues.is_empty(), "{name}");
        }
    }

    #[tokio::test]
    async fn unticking_one_place_of_two_keeps_the_values() {
        let (_dir, db) = lab_with_issues().await;
        field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();

        field_choice_set(&db, ZONE.into(), false, true)
            .await
            .unwrap();

        assert_eq!(
            custom_of(&db, "2").await,
            Some(serde_json::json!({ ZONE: "Paris" }))
        );
        assert_eq!(
            raw_of(&db, "1").await,
            serde_json::json!({
                "id": "1",
                "renderedFields": { ZONE: "<p>Berlin</p>", ALPHA: "<p>Ann</p>" }
            })
        );
    }

    #[tokio::test]
    async fn ticking_a_forgotten_field_again_asks_for_a_full_sync() {
        let (_dir, db) = lab_with_issues().await;
        field_choice_set(&db, ZONE.into(), true, true)
            .await
            .unwrap();
        field_choice_set(&db, ZONE.into(), false, false)
            .await
            .unwrap();

        let saved = field_choice_set(&db, ZONE.into(), false, true)
            .await
            .unwrap();

        assert!(saved.newly_fetched);
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
