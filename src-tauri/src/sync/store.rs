//! The cache side of the sync: what it writes inside `Db::write` transactions, and the few reads it needs.

use rusqlite::{Connection, OptionalExtension, params};

pub use super::fields::{fetched_fields, store_fields};
pub use super::news::{Pass, page_pass};
pub use super::page::store_page;

use crate::error::AppResult;
use crate::jira::dto::{Status, Version};

/// The `sync_state` row of the whole scope.
pub const SCOPE_ROW: &str = "scope";
/// `sync_state` rows of single projects are this prefix plus the project id.
pub const PROJECT_ROW_PREFIX: &str = "full:";

pub fn store_statuses(conn: &Connection, statuses: &[Status]) -> AppResult<()> {
    for status in statuses {
        conn.execute(
            "INSERT INTO cache.status (id, name, category) VALUES (?1, ?2, ?3)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, category = excluded.category",
            params![status.id, status.name, status.status_category.key],
        )?;
    }
    Ok(())
}

/// Replaces a project's versions with Jira's list.
pub fn store_versions(conn: &Connection, project_id: &str, versions: &[Version]) -> AppResult<()> {
    conn.execute(
        "DELETE FROM cache.version WHERE project_id = ?1",
        [project_id],
    )?;
    for version in versions {
        conn.execute(
            "INSERT INTO cache.version (id, project_id, name, release_date, released)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (id) DO UPDATE SET project_id = excluded.project_id, name = excluded.name,
               release_date = excluded.release_date, released = excluded.released",
            params![
                version.id,
                project_id,
                version.name,
                version.release_date,
                version.released
            ],
        )?;
    }
    Ok(())
}

/// When the whole scope was last synced (UTC), if ever.
pub fn last_full_at(conn: &Connection) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT last_full_at FROM cache.sync_state WHERE scope_id = ?1",
            [SCOPE_ROW],
            |row| row.get(0),
        )
        .optional()?
        .flatten())
}

/// Where the next delta sync starts: the newest `updated` seen.
pub fn last_delta_at(conn: &Connection) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT last_delta_at FROM cache.sync_state WHERE scope_id = ?1",
            [SCOPE_ROW],
            |row| row.get(0),
        )
        .optional()?
        .flatten())
}

/// Moves the delta cursor forward (never back) once a delta finished.
pub fn record_delta(conn: &Connection, newest: Option<&str>) -> AppResult<()> {
    conn.execute(
        "UPDATE cache.sync_state SET last_delta_at = MAX(COALESCE(last_delta_at, ''), COALESCE(?2, '')), last_error = NULL
         WHERE scope_id = ?1",
        params![SCOPE_ROW, newest],
    )?;
    Ok(())
}

pub fn record_reconcile(conn: &Connection, now: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE cache.sync_state SET last_reconcile_at = ?2 WHERE scope_id = ?1",
        params![SCOPE_ROW, now],
    )?;
    Ok(())
}

/// Removes the issue with its search row, seen mark and activity; comments and changes go with it.
pub fn forget_issue(conn: &Connection, id: &str) -> AppResult<bool> {
    conn.execute(
        "DELETE FROM cache.issue_fts WHERE rowid = CAST(?1 AS INTEGER)",
        [id],
    )?;
    conn.execute("DELETE FROM issue_seen WHERE issue_id = ?1", [id])?;
    conn.execute("DELETE FROM issue_activity WHERE issue_id = ?1", [id])?;
    Ok(conn.execute("DELETE FROM cache.issue WHERE id = ?1", [id])? > 0)
}

/// Forgets the cached issues of the `types` (by name), as `forget_issue` does. Returns their ids.
pub fn forget_types(conn: &Connection, types: &[String]) -> AppResult<Vec<String>> {
    let ids: Vec<String> = {
        let mut query = conn.prepare(
            "SELECT id FROM cache.issue WHERE issue_type IN (SELECT value FROM json_each(?1))",
        )?;
        query
            .query_map([serde_json::to_string(types)?], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    for id in &ids {
        forget_issue(conn, id)?;
    }
    Ok(ids)
}

/// Deletes a project's cached issues (their comments, changes and links go with them) and its sync record, so
/// tracking it again fetches it anew; what the owner has read stays. Returns the deleted issues' ids.
pub fn forget_project(conn: &Connection, project_id: &str) -> AppResult<Vec<String>> {
    let ids: Vec<String> = {
        let mut query = conn.prepare("SELECT id FROM cache.issue WHERE project_id = ?1")?;
        query
            .query_map([project_id], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    conn.execute(
        "DELETE FROM cache.issue_fts WHERE rowid IN
           (SELECT CAST(id AS INTEGER) FROM cache.issue WHERE project_id = ?1)",
        [project_id],
    )?;
    conn.execute(
        "DELETE FROM cache.issue WHERE project_id = ?1",
        [project_id],
    )?;
    conn.execute(
        "DELETE FROM cache.sync_state WHERE scope_id = ?1 || ?2",
        params![PROJECT_ROW_PREFIX, project_id],
    )?;
    Ok(ids)
}

/// The most recently updated live issues, for the comment prefetch after a delta.
pub fn recently_updated(conn: &Connection, limit: usize) -> AppResult<Vec<String>> {
    let mut query = conn
        .prepare("SELECT id FROM cache.issue WHERE gone = 0 ORDER BY updated_at DESC LIMIT ?1")?;
    let ids = query.query_map([limit as i64], |r| r.get(0))?;
    Ok(ids.collect::<Result<_, _>>()?)
}

pub fn has_project_sync(conn: &Connection, project_id: &str) -> AppResult<bool> {
    let found: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cache.sync_state WHERE scope_id = ?1 || ?2 AND last_full_at IS NOT NULL",
        params![PROJECT_ROW_PREFIX, project_id],
        |row| row.get(0),
    )?;
    Ok(found > 0)
}

pub fn record_project_sync(
    conn: &Connection,
    project_id: &str,
    jql: &str,
    now: &str,
    approx_total: u64,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.sync_state (scope_id, project_id, jql, last_full_at, approx_total, last_error)
         VALUES (?1 || ?2, ?2, ?3, ?4, ?5, NULL)
         ON CONFLICT (scope_id) DO UPDATE SET project_id = excluded.project_id, jql = excluded.jql,
           last_full_at = excluded.last_full_at, approx_total = excluded.approx_total, last_error = NULL",
        params![PROJECT_ROW_PREFIX, project_id, jql, now, approx_total as i64],
    )?;
    Ok(())
}

/// `newest` (the newest `updated` stored) becomes the first delta cursor; `None` keeps the old one.
pub fn record_scope_sync(
    conn: &Connection,
    jql: &str,
    now: &str,
    newest: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.sync_state (scope_id, jql, last_full_at, last_delta_at, last_error)
         VALUES (?1, ?2, ?3, ?4, NULL)
         ON CONFLICT (scope_id) DO UPDATE SET jql = excluded.jql, last_full_at = excluded.last_full_at,
           last_delta_at = COALESCE(excluded.last_delta_at, last_delta_at), last_error = NULL",
        params![SCOPE_ROW, jql, now, newest],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    const NW: &str = "10001";
    const OPS: &str = "10002";

    fn issue_sql(id: u32, project: &str) -> String {
        format!(
            "INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
                                     created_at, updated_at, synced_at, gone)
             VALUES ('{id}', 'K-{id}', '{project}', 'Issue {id}', 'Task', 's1', 'new',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 0);
             INSERT INTO cache.issue_fts (rowid, key, summary, description, comments)
             VALUES ({id}, 'K-{id}', 'Issue {id}', '', '');
             INSERT INTO cache.comment (id, issue_id, created_at, updated_at)
             VALUES ('c{id}', '{id}', '2026-01-02T00:00:00Z', '2026-01-02T00:00:00Z');
             INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
             VALUES ('{id}', 'priority', 'High', 'Low', '2026-01-02T00:00:00Z');
             INSERT INTO issue_seen (issue_id, seen_at) VALUES ('{id}', '2026-01-03T00:00:00Z');"
        )
    }

    async fn lab() -> (tempfile::TempDir, Db) {
        let dir = crate::test_support::temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        let sql = [
            "INSERT INTO cache.space (id, key, name, raw) VALUES
               ('10001', 'NW', 'Website relaunch', '{}'), ('10002', 'OPS', 'Operations', '{}');"
                .to_string(),
            issue_sql(1, NW),
            issue_sql(2, NW),
            issue_sql(3, OPS),
        ]
        .concat();
        db.write(move |tx, _| {
            tx.execute_batch(&sql)?;
            record_project_sync(tx, NW, "project = NW", "2026-01-04T00:00:00Z", 2)?;
            record_project_sync(tx, OPS, "project = OPS", "2026-01-04T00:00:00Z", 1)?;
            record_scope_sync(tx, "project in (NW, OPS)", "2026-01-04T00:00:00Z", None)
        })
        .await
        .unwrap();
        (dir, db)
    }

    async fn count(db: &Db, sql: &'static str) -> i64 {
        db.read(move |conn| Ok(conn.query_row(sql, [], |row| row.get(0))?))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn forgetting_a_project_returns_and_deletes_its_issues_with_everything_attached() {
        let (_dir, db) = lab().await;

        let mut ids = db.write(|tx, _| forget_project(tx, NW)).await.unwrap();
        ids.sort();

        assert_eq!(ids, vec!["1".to_string(), "2".to_string()]);
        for sql in [
            "SELECT COUNT(*) FROM cache.issue WHERE project_id = '10001'",
            "SELECT COUNT(*) FROM cache.issue_fts WHERE rowid IN (1, 2)",
            "SELECT COUNT(*) FROM cache.comment WHERE issue_id IN ('1', '2')",
            "SELECT COUNT(*) FROM cache.issue_change WHERE issue_id IN ('1', '2')",
            "SELECT COUNT(*) FROM cache.sync_state WHERE scope_id = 'full:10001'",
        ] {
            assert_eq!(count(&db, sql).await, 0, "{sql}");
        }
    }

    #[tokio::test]
    async fn forgetting_a_project_keeps_read_marks_other_projects_and_the_scope_record() {
        let (_dir, db) = lab().await;

        db.write(|tx, _| forget_project(tx, NW)).await.unwrap();

        for (sql, expected) in [
            (
                "SELECT COUNT(*) FROM issue_seen WHERE issue_id IN ('1', '2')",
                2,
            ),
            (
                "SELECT COUNT(*) FROM cache.issue WHERE project_id = '10002'",
                1,
            ),
            ("SELECT COUNT(*) FROM cache.issue_fts WHERE rowid = 3", 1),
            ("SELECT COUNT(*) FROM cache.comment WHERE issue_id = '3'", 1),
            (
                "SELECT COUNT(*) FROM cache.issue_change WHERE issue_id = '3'",
                1,
            ),
            (
                "SELECT COUNT(*) FROM cache.sync_state WHERE scope_id = 'full:10002'",
                1,
            ),
            (
                "SELECT COUNT(*) FROM cache.sync_state WHERE scope_id = 'scope'",
                1,
            ),
        ] {
            assert_eq!(count(&db, sql).await, expected, "{sql}");
        }
    }

    async fn stamp_issues_1_and_2(db: &Db) {
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO issue_activity (issue_id, viewed_at, interacted_at) VALUES
                   ('1', '2026-01-05T00:00:00.000Z', '2026-01-05T00:00:00.000Z'),
                   ('2', NULL, '2026-01-06T00:00:00.000Z');",
            )?;
            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn forgetting_an_issue_deletes_its_activity_and_keeps_the_others() {
        let (_dir, db) = lab().await;
        stamp_issues_1_and_2(&db).await;

        let removed = db.write(|tx, _| forget_issue(tx, "1")).await.unwrap();

        assert!(removed);
        assert_eq!(
            count(
                &db,
                "SELECT COUNT(*) FROM issue_activity WHERE issue_id = '1'"
            )
            .await,
            0
        );
        assert_eq!(
            count(
                &db,
                "SELECT COUNT(*) FROM issue_activity WHERE issue_id = '2'"
            )
            .await,
            1
        );
    }

    #[tokio::test]
    async fn forgetting_a_project_keeps_the_activity_of_its_issues() {
        let (_dir, db) = lab().await;
        stamp_issues_1_and_2(&db).await;

        db.write(|tx, _| forget_project(tx, NW)).await.unwrap();

        assert_eq!(count(&db, "SELECT COUNT(*) FROM issue_activity").await, 2);
    }

    async fn type_issues(db: &Db) {
        db.write(|tx, _| {
            tx.execute_batch(
                "UPDATE cache.issue SET issue_type = 'Test' WHERE id IN ('1', '3');
                 UPDATE cache.issue SET issue_type = 'Story' WHERE id = '2';",
            )?;
            Ok(())
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn forgetting_types_returns_and_deletes_their_issues_with_everything_attached() {
        let (_dir, db) = lab().await;
        type_issues(&db).await;
        stamp_issues_1_and_2(&db).await;

        let mut ids = db
            .write(|tx, _| forget_types(tx, &["Test".to_string()]))
            .await
            .unwrap();
        ids.sort();

        assert_eq!(ids, vec!["1".to_string(), "3".to_string()]);
        for sql in [
            "SELECT COUNT(*) FROM cache.issue WHERE id IN ('1', '3')",
            "SELECT COUNT(*) FROM cache.issue_fts WHERE rowid IN (1, 3)",
            "SELECT COUNT(*) FROM cache.comment WHERE issue_id IN ('1', '3')",
            "SELECT COUNT(*) FROM cache.issue_change WHERE issue_id IN ('1', '3')",
            "SELECT COUNT(*) FROM issue_seen WHERE issue_id IN ('1', '3')",
            "SELECT COUNT(*) FROM issue_activity WHERE issue_id IN ('1', '3')",
        ] {
            assert_eq!(count(&db, sql).await, 0, "{sql}");
        }
    }

    #[tokio::test]
    async fn forgetting_types_keeps_issues_of_other_types_with_their_marks() {
        let (_dir, db) = lab().await;
        type_issues(&db).await;
        stamp_issues_1_and_2(&db).await;

        db.write(|tx, _| forget_types(tx, &["Test".to_string()]))
            .await
            .unwrap();

        for sql in [
            "SELECT COUNT(*) FROM cache.issue WHERE id = '2'",
            "SELECT COUNT(*) FROM cache.comment WHERE issue_id = '2'",
            "SELECT COUNT(*) FROM issue_seen WHERE issue_id = '2'",
            "SELECT COUNT(*) FROM issue_activity WHERE issue_id = '2'",
        ] {
            assert_eq!(count(&db, sql).await, 1, "{sql}");
        }
    }

    #[tokio::test]
    async fn forgetting_types_without_matching_issues_returns_nothing() {
        let (_dir, db) = lab().await;
        type_issues(&db).await;

        for types in [vec![], vec!["Bug".to_string()]] {
            let ids = db
                .write(move |tx, _| forget_types(tx, &types))
                .await
                .unwrap();
            assert_eq!(ids, Vec::<String>::new());
        }
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.issue").await, 3);
    }

    #[tokio::test]
    async fn forgetting_a_project_without_issues_returns_nothing() {
        let (_dir, db) = lab().await;

        let ids = db.write(|tx, _| forget_project(tx, "99999")).await.unwrap();

        assert_eq!(ids, Vec::<String>::new());
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.issue").await, 3);
    }
}
