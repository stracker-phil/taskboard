//! What reconciliation keeps and lets go: issues gone from Jira, issues outside the scope, old change records.

use rusqlite::{Connection, params};

use crate::db::Changes;
use crate::error::AppResult;

use super::store::forget_issue;

/// A cached issue Jira still lists, and when reconciliation last found it outside the scope.
pub struct LiveIssue {
    pub id: String,
    pub left_scope_at: Option<String>,
}

pub fn live_issues(conn: &Connection) -> AppResult<Vec<LiveIssue>> {
    let mut query = conn.prepare("SELECT id, left_scope_at FROM cache.issue WHERE gone = 0")?;
    let issues = query.query_map([], |r| {
        Ok(LiveIssue {
            id: r.get(0)?,
            left_scope_at: r.get(1)?,
        })
    })?;
    Ok(issues.collect::<Result<_, _>>()?)
}

/// `Some(now)` marks the issues as found outside the scope at `now`; `None` as back inside it.
pub fn mark_left_scope(conn: &Connection, ids: &[String], at: Option<&str>) -> AppResult<()> {
    conn.execute(
        "UPDATE cache.issue SET left_scope_at = ?2 WHERE id IN (SELECT value FROM json_each(?1))",
        params![serde_json::to_string(ids)?, at],
    )?;
    Ok(())
}

/// Marks issues Jira no longer returns as gone and records when; they leave every list at once.
pub fn mark_gone(
    conn: &Connection,
    changes: &mut Changes,
    ids: &[String],
    now: &str,
) -> AppResult<()> {
    for id in ids {
        let marked = conn.execute(
            "UPDATE cache.issue SET gone = 1 WHERE id = ?1 AND gone = 0",
            [id],
        )?;
        if marked > 0 {
            conn.execute(
                "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
                 VALUES (?1, 'gone', NULL, NULL, ?2)",
                params![id, now],
            )?;
            changes.issue_changed(id.clone());
        }
    }
    Ok(())
}

/// Deletes issues gone for longer than `before` (with their comments, changes and seen rows).
pub fn purge_gone(conn: &Connection, before: &str) -> AppResult<usize> {
    let ids: Vec<String> = {
        let mut query = conn.prepare(
            "SELECT i.id FROM cache.issue i WHERE i.gone = 1 AND NOT EXISTS (
               SELECT 1 FROM cache.issue_change c WHERE c.issue_id = i.id AND c.field = 'gone' AND c.changed_at >= ?1)",
        )?;
        query
            .query_map([before], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for id in &ids {
        forget_issue(conn, id)?;
    }
    Ok(ids.len())
}

/// Deletes the recorded changes from before `before` that the owner has seen. `created` stays (the activity
/// always shows it), and so does `gone` (`purge_gone` counts from it).
pub fn prune_seen_changes(conn: &Connection, before: &str) -> AppResult<usize> {
    Ok(conn.execute(
        "DELETE FROM cache.issue_change
         WHERE changed_at < ?1 AND field NOT IN ('created', 'gone')
           AND changed_at <= (SELECT s.seen_at FROM cache.issue_seen s WHERE s.issue_id = issue_change.issue_id)",
        [before],
    )?)
}
