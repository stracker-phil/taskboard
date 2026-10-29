//! Which custom fields the pickers offer: the marks `store_field_usage` writes, and the Columns catalogue and
//! the filter editor's field list that leave unused fields out unless the owner already uses them.

use std::collections::BTreeSet;

use tempfile::TempDir;

use taskboard_lib::db::Db;
use taskboard_lib::lists::filter_options;
use taskboard_lib::sync::store::store_field_usage;
use taskboard_lib::test_support::temp_config_dir;
use taskboard_lib::views::columns_catalogue;

const STORY_POINTS: &str = "customfield_10016";
const REVIEWER: &str = "customfield_10071";
const COST_CENTRE: &str = "customfield_10090";
const LEGAL_APPROVAL: &str = "customfield_10091";

async fn execute(db: &Db, sql: &'static str) {
    db.write(move |tx, _| {
        tx.execute_batch(sql)?;
        Ok(())
    })
    .await
    .unwrap();
}

/// A database with the demo site's fields, all still unsampled, and one filter list with a view.
async fn lab() -> (TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    execute(
        &db,
        "INSERT INTO cache.field (id, name, type) VALUES
           ('summary', 'Summary', 'string'),
           ('customfield_10016', 'Story points', 'number'),
           ('customfield_10071', 'Reviewer', 'user'),
           ('customfield_10090', 'Cost centre', 'option'),
           ('customfield_10091', 'Legal approval date', 'date'),
           ('customfield_10019', 'Rank', 'any');
         INSERT INTO filter_list (kind, name) VALUES ('list', 'Open bugs');
         INSERT INTO view (list_id, name, layout, columns, group_by, sort_by, sort_order)
           VALUES ((SELECT id FROM filter_list WHERE name = 'Open bugs'), 'All issues', 'list',
                   '[\"status\"]', 'status', 'key', 0)",
    )
    .await;
    (dir, db)
}

async fn mark_used(db: &Db, used: &[&str]) {
    let used: BTreeSet<String> = used.iter().map(|id| id.to_string()).collect();
    db.write(move |tx, _| store_field_usage(tx, &used))
        .await
        .unwrap();
}

async fn marks(db: &Db) -> Vec<(String, i64)> {
    db.read(|conn| {
        let mut query = conn.prepare("SELECT id, used FROM cache.field ORDER BY id")?;
        let found = query
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(found)
    })
    .await
    .unwrap()
}

fn marked(items: &[(&str, i64)]) -> Vec<(String, i64)> {
    items
        .iter()
        .map(|(id, used)| (id.to_string(), *used))
        .collect()
}

/// The custom fields the Columns popover offers, then the ones the filter editor offers.
async fn offered(db: &Db) -> (Vec<String>, Vec<String>) {
    let columns = columns_catalogue(db)
        .await
        .unwrap()
        .into_iter()
        .filter(|c| c.custom)
        .map(|c| c.id)
        .collect();
    let filters = filter_options(db)
        .await
        .unwrap()
        .fields
        .into_iter()
        .filter(|f| f.id.starts_with("customfield_"))
        .map(|f| f.id)
        .collect();
    (columns, filters)
}

async fn assert_offered(db: &Db, expected: &[&str]) {
    let expected: Vec<String> = expected.iter().map(|id| id.to_string()).collect();
    let (columns, filters) = offered(db).await;
    assert_eq!(columns, expected, "columns");
    assert_eq!(filters, expected, "filter fields");
}

mod the_marks {
    use super::*;

    #[tokio::test]
    async fn the_given_custom_fields_are_used_and_the_other_custom_fields_are_not() {
        let (_dir, db) = lab().await;

        mark_used(&db, &[STORY_POINTS, REVIEWER]).await;

        assert_eq!(
            marks(&db).await,
            marked(&[
                ("customfield_10016", 1),
                ("customfield_10019", 0),
                ("customfield_10071", 1),
                ("customfield_10090", 0),
                ("customfield_10091", 0),
                ("summary", 1),
            ])
        );
    }

    #[tokio::test]
    async fn a_later_sample_replaces_the_earlier_one() {
        let (_dir, db) = lab().await;
        mark_used(&db, &[STORY_POINTS, REVIEWER]).await;

        mark_used(&db, &[COST_CENTRE]).await;

        assert_eq!(
            marks(&db).await,
            marked(&[
                ("customfield_10016", 0),
                ("customfield_10019", 0),
                ("customfield_10071", 0),
                ("customfield_10090", 1),
                ("customfield_10091", 0),
                ("summary", 1),
            ])
        );
    }

    #[tokio::test]
    async fn an_empty_sample_marks_every_custom_field_unused_and_leaves_system_fields() {
        let (_dir, db) = lab().await;

        mark_used(&db, &[]).await;

        let used: Vec<String> = marks(&db)
            .await
            .into_iter()
            .filter(|(_, used)| *used == 1)
            .map(|(id, _)| id)
            .collect();
        assert_eq!(used, ["summary"]);
    }

    #[tokio::test]
    async fn an_id_the_site_does_not_have_adds_no_row() {
        let (_dir, db) = lab().await;

        mark_used(&db, &["customfield_99999"]).await;

        assert_eq!(marks(&db).await.len(), 6);
    }
}

mod the_pickers {
    use super::*;

    #[tokio::test]
    async fn every_custom_field_is_offered_before_any_sampling() {
        let (_dir, db) = lab().await;

        assert_offered(&db, &[COST_CENTRE, LEGAL_APPROVAL, REVIEWER, STORY_POINTS]).await;
    }

    #[tokio::test]
    async fn a_field_no_tracked_issue_fills_is_left_out() {
        let (_dir, db) = lab().await;

        mark_used(&db, &[STORY_POINTS, REVIEWER]).await;

        assert_offered(&db, &[REVIEWER, STORY_POINTS]).await;
    }

    #[tokio::test]
    async fn a_field_no_tracked_issue_fills_stays_when_it_is_shown_in_the_field_grid() {
        let (_dir, db) = lab().await;
        mark_used(&db, &[STORY_POINTS, REVIEWER]).await;

        execute(
            &db,
            "UPDATE cache.field SET shown = 1 WHERE id = 'customfield_10090'",
        )
        .await;

        assert_offered(&db, &[COST_CENTRE, REVIEWER, STORY_POINTS]).await;
    }

    #[tokio::test]
    async fn a_field_no_tracked_issue_fills_stays_when_a_view_has_it_as_a_column() {
        let (_dir, db) = lab().await;
        mark_used(&db, &[STORY_POINTS, REVIEWER]).await;

        execute(
            &db,
            "UPDATE view SET columns = '[\"status\",\"customfield_10091\"]'",
        )
        .await;

        assert_offered(&db, &[LEGAL_APPROVAL, REVIEWER, STORY_POINTS]).await;
    }

    #[tokio::test]
    async fn a_field_no_tracked_issue_fills_stays_when_a_filter_list_has_a_condition_on_it() {
        let (_dir, db) = lab().await;
        mark_used(&db, &[STORY_POINTS, REVIEWER]).await;

        execute(
            &db,
            "UPDATE filter_list SET filter = '{\"conditions\":[
               {\"field\":\"type\",\"op\":\"is\",\"value\":\"Bug\"},
               {\"field\":\"customfield_10090\",\"op\":\"is\",\"value\":\"CC-100\"}]}'",
        )
        .await;

        assert_offered(&db, &[COST_CENTRE, REVIEWER, STORY_POINTS]).await;
    }

    #[tokio::test]
    async fn a_view_without_columns_and_a_list_without_a_filter_keep_nothing_alive() {
        let (_dir, db) = lab().await;
        mark_used(&db, &[STORY_POINTS]).await;
        execute(&db, "UPDATE view SET columns = NULL").await;

        assert_offered(&db, &[STORY_POINTS]).await;
    }

    #[tokio::test]
    async fn internal_fields_stay_out_even_when_a_view_shows_them() {
        let (_dir, db) = lab().await;
        execute(&db, "UPDATE view SET columns = '[\"customfield_10019\"]'").await;

        assert_offered(&db, &[COST_CENTRE, LEGAL_APPROVAL, REVIEWER, STORY_POINTS]).await;
    }
}
