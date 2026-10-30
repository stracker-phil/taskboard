//! The cache side of the sync: what it writes inside `Db::write` transactions, and the few reads it needs.

use rusqlite::{Connection, OptionalExtension, params};

pub use super::fields::{fetched_fields, shown_fields, store_field_usage, store_fields};

use super::diff::{Snapshot, diff};
use super::map::{FieldIds, IssueRow, Mapped, map_issue};
use crate::db::Changes;
use crate::error::AppResult;
use crate::jira::dto::{Issue, Status, Version};

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

/// Removes the issue with its search row; comments, changes and seen rows go with it.
pub fn forget_issue(conn: &Connection, id: &str) -> AppResult<bool> {
    conn.execute(
        "DELETE FROM cache.issue_fts WHERE rowid = CAST(?1 AS INTEGER)",
        [id],
    )?;
    Ok(conn.execute("DELETE FROM cache.issue WHERE id = ?1", [id])? > 0)
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
/// announce nothing. On a project's first sync (`first`) changes aren't recorded and every issue counts as
/// seen at `now`.
pub fn store_page(
    conn: &Connection,
    changes: &mut Changes,
    issues: &[Issue],
    ids: &FieldIds,
    first: bool,
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
        if store_issue(conn, &mapped, first, now)? {
            changes.issue_changed(&mapped.issue.id);
        }
    }
    Ok(newest)
}

struct CachedIssue {
    snapshot: Snapshot,
    custom: Option<String>,
    raw: Option<String>,
    gone: bool,
}

fn cached_issue(conn: &Connection, id: &str) -> AppResult<Option<CachedIssue>> {
    Ok(conn
        .query_row(
            "SELECT i.status_id, i.assignee_id, i.due_date, i.priority, i.comment_count, i.custom, b.raw, i.gone
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
                })
            },
        )
        .optional()?)
}

/// `false` when the cache already holds this exact version.
fn store_issue(conn: &Connection, mapped: &Mapped, first: bool, now: &str) -> AppResult<bool> {
    let row = &mapped.issue;
    let cached = cached_issue(conn, &row.id)?;
    if cached.as_ref().is_some_and(|c| {
        !c.gone && c.custom == row.custom && c.raw.as_deref() == Some(row.raw.as_str())
    }) {
        return Ok(false);
    }
    upsert_issue(conn, row, now)?;
    upsert_body(conn, row)?;
    store_users(conn, mapped)?;
    store_comments(conn, mapped, now)?;
    replace_search_doc(conn, mapped)?;
    if first {
        conn.execute(
            "INSERT INTO cache.issue_seen (issue_id, seen_at) VALUES (?1, ?2)
             ON CONFLICT (issue_id) DO NOTHING",
            params![row.id, now],
        )?;
    } else {
        record_changes(conn, cached.as_ref().map(|c| &c.snapshot), row)?;
    }
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
        conn.execute("DELETE FROM cache.comment WHERE issue_id = ?1", [issue_id])?;
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

fn record_changes(conn: &Connection, cached: Option<&Snapshot>, row: &IssueRow) -> AppResult<()> {
    for change in diff(cached, row) {
        conn.execute(
            "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                row.id,
                change.field,
                change.old_value,
                change.new_value,
                change.changed_at
            ],
        )?;
    }
    Ok(())
}
