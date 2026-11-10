//! The cache side of the sync: what it writes inside `Db::write` transactions, and the few reads it needs.

use rusqlite::{Connection, OptionalExtension, params};

pub use super::fields::{fetched_fields, store_fields};

use super::diff::Snapshot;
use super::map::{FieldIds, IssueRow, Mapped, map_issue};
use super::mentions::record_mentions;
use super::news::{Arrival, new_comment_authors, record_news};
pub use super::news::{Pass, page_pass};
use crate::db::Changes;
use crate::error::AppResult;
use crate::jira::dto::{Issue, Status, Version};
use crate::writes;

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

/// Upserts a page of issues and returns the newest `updated` on it. Unchanged issues are skipped, so they
/// announce nothing. Changes by `me` (the connected account) that the page shows aren't news.
pub fn store_page(
    conn: &Connection,
    changes: &mut Changes,
    issues: &[Issue],
    ids: &FieldIds,
    pass: Pass,
    me: Option<&str>,
    now: &str,
) -> AppResult<Option<String>> {
    let mut newest: Option<String> = None;
    for issue in issues {
        let Some(mapped) = map_issue(issue, ids) else {
            log::warn!(
                "Skipped issue {} without project, status, dates or a numeric id",
                issue.id
            );
            continue;
        };
        if newest.as_deref() < Some(mapped.issue.updated_at.as_str()) {
            newest = Some(mapped.issue.updated_at.clone());
        }
        if store_issue(conn, changes, &mapped, pass, me, now)? {
            changes.issue_changed(&mapped.issue.id);
        }
    }
    Ok(newest)
}

struct CachedIssue {
    snapshot: Snapshot,
    updated_at: String,
    custom: Option<String>,
    raw: Option<String>,
    gone: bool,
}

fn cached_issue(conn: &Connection, id: &str) -> AppResult<Option<CachedIssue>> {
    Ok(conn
        .query_row(
            "SELECT i.status_id, i.assignee_id, i.due_date, i.priority, i.comment_count, i.custom, b.raw, i.gone,
               i.updated_at
             FROM cache.issue i LEFT JOIN cache.issue_body b ON b.issue_id = i.id WHERE i.id = ?1",
            [id],
            |r| {
                Ok(CachedIssue {
                    snapshot: Snapshot {
                        status_id: r.get(0)?,
                        assignee_id: r.get(1)?,
                        due_date: r.get(2)?,
                        priority: r.get(3)?,
                        comment_count: r.get(4)?,
                    },
                    custom: r.get(5)?,
                    raw: r.get(6)?,
                    gone: r.get(7)?,
                    updated_at: r.get(8)?,
                })
            },
        )
        .optional()?)
}

/// `false` when the cache already holds this version or a newer one.
fn store_issue(
    conn: &Connection,
    changes: &mut Changes,
    mapped: &Mapped,
    pass: Pass,
    me: Option<&str>,
    now: &str,
) -> AppResult<bool> {
    let cached = cached_issue(conn, &mapped.issue.id)?;
    if cached.as_ref().is_some_and(|c| {
        let same =
            c.custom == mapped.issue.custom && c.raw.as_deref() == Some(mapped.issue.raw.as_str());
        !c.gone && (same || c.updated_at > mapped.issue.updated_at)
    }) {
        return Ok(false);
    }
    let mut row = mapped.issue.clone();
    writes::overlay(conn, &mut row)?;
    // Before the page's comments are stored, which would make them all known.
    let new_comments = new_comment_authors(conn, mapped)?;
    upsert_issue(conn, &row, now)?;
    upsert_body(conn, &row)?;
    store_users(conn, mapped)?;
    if pass.records_news() {
        record_mentions(conn, mapped, me)?;
    }
    store_comments(conn, mapped, now)?;
    replace_search_doc(conn, mapped)?;
    record_news(
        conn,
        changes,
        Arrival {
            row: &row,
            previous: cached.as_ref().map(|c| &c.snapshot),
            new_comments: &new_comments,
            pass,
            me,
            now,
        },
    )?;
    Ok(true)
}

fn upsert_issue(conn: &Connection, row: &IssueRow, now: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.issue (id, key, project_id, parent_id, fix_versions, epic_color, summary,
           issue_type, type_is_subtask, is_epic, status_id, status_category, priority, assignee_id,
           reporter_id, sprint, sprint_id, sprint_state, labels, due_date, created_at, updated_at,
           resolved_at, comment_count, custom, synced_at, gone)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
           ?20, ?21, ?22, ?23, ?24, ?25, ?26, 0)
         ON CONFLICT (id) DO UPDATE SET key = excluded.key, project_id = excluded.project_id,
           parent_id = excluded.parent_id, fix_versions = excluded.fix_versions,
           epic_color = excluded.epic_color, summary = excluded.summary, issue_type = excluded.issue_type,
           type_is_subtask = excluded.type_is_subtask, is_epic = excluded.is_epic,
           status_id = excluded.status_id, status_category = excluded.status_category,
           priority = excluded.priority, assignee_id = excluded.assignee_id,
           reporter_id = excluded.reporter_id, sprint = excluded.sprint, sprint_id = excluded.sprint_id,
           sprint_state = excluded.sprint_state, labels = excluded.labels, due_date = excluded.due_date,
           created_at = excluded.created_at, updated_at = excluded.updated_at,
           resolved_at = excluded.resolved_at, comment_count = excluded.comment_count,
           custom = excluded.custom, synced_at = excluded.synced_at, gone = 0",
        params![
            row.id,
            row.key,
            row.project_id,
            row.parent_id,
            row.fix_versions,
            row.epic_color,
            row.summary,
            row.issue_type,
            row.type_is_subtask,
            row.is_epic,
            row.status_id,
            row.status_category,
            row.priority,
            row.assignee_id,
            row.reporter_id,
            row.sprint,
            row.sprint_id,
            row.sprint_state,
            row.labels,
            row.due_date,
            row.created_at,
            row.updated_at,
            row.resolved_at,
            row.comment_count,
            row.custom,
            now
        ],
    )?;
    Ok(())
}

fn upsert_body(conn: &Connection, row: &IssueRow) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.issue_body (issue_id, description_html, description_adf, raw)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (issue_id) DO UPDATE SET description_html = excluded.description_html,
           description_adf = excluded.description_adf, raw = excluded.raw",
        params![row.id, row.description_html, row.description_adf, row.raw],
    )?;
    Ok(())
}

fn store_users(conn: &Connection, mapped: &Mapped) -> AppResult<()> {
    for user in &mapped.users {
        conn.execute(
            "INSERT INTO cache.user (account_id, name, avatar_url) VALUES (?1, ?2, ?3)
             ON CONFLICT (account_id) DO UPDATE SET name = excluded.name, avatar_url = excluded.avatar_url",
            params![user.account_id, user.name, user.avatar_url],
        )?;
    }
    Ok(())
}

/// The search's comments; when they are all of the issue's, they replace the cached ones.
fn store_comments(conn: &Connection, mapped: &Mapped, now: &str) -> AppResult<()> {
    let issue_id = &mapped.issue.id;
    if mapped.issue.comments_complete {
        conn.execute(
            "DELETE FROM cache.comment WHERE issue_id = ?1 AND id NOT LIKE 'pending:%'",
            [issue_id],
        )?;
        conn.execute(
            "UPDATE cache.issue SET comments_synced_at = ?2 WHERE id = ?1",
            params![issue_id, now],
        )?;
    }
    for comment in &mapped.comments {
        conn.execute(
            "INSERT INTO cache.comment (id, issue_id, author_id, body_html, body_text, created_at, updated_at, body_adf)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT (id) DO UPDATE SET issue_id = excluded.issue_id, author_id = excluded.author_id,
               body_html = excluded.body_html, body_text = excluded.body_text,
               created_at = excluded.created_at, updated_at = excluded.updated_at, body_adf = excluded.body_adf",
            params![
                comment.id,
                comment.issue_id,
                comment.author_id,
                comment.body_html,
                comment.body_text,
                comment.created_at,
                comment.updated_at,
                comment.body_adf
            ],
        )?;
    }
    Ok(())
}

fn replace_search_doc(conn: &Connection, mapped: &Mapped) -> AppResult<()> {
    let doc = &mapped.fts;
    conn.execute("DELETE FROM cache.issue_fts WHERE rowid = ?1", [doc.rowid])?;
    conn.execute(
        "INSERT INTO cache.issue_fts (rowid, key, summary, description, comments)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            doc.rowid,
            doc.key,
            doc.summary,
            doc.description,
            doc.comments
        ],
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

    #[tokio::test]
    async fn forgetting_a_project_without_issues_returns_nothing() {
        let (_dir, db) = lab().await;

        let ids = db.write(|tx, _| forget_project(tx, "99999")).await.unwrap();

        assert_eq!(ids, Vec::<String>::new());
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.issue").await, 3);
    }
}
