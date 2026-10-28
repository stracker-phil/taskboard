//! Which recorded changes the owner hasn't seen: those after the issue's `issue_seen` time, or all of them
//! when it has none (the first sync marks every issue it finds as seen).

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppResult;

/// An issue (alias `i`) with a change recorded after the owner last saw it.
pub const UNSEEN: &str = "EXISTS (SELECT 1 FROM cache.issue_change c
    LEFT JOIN cache.issue_seen s ON s.issue_id = c.issue_id
    WHERE c.issue_id = i.id AND c.field != 'gone' AND (s.seen_at IS NULL OR c.changed_at > s.seen_at))";

/// One row of `issue_change`.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    /// UTC, sortable as text.
    pub at: String,
}

fn change(r: &rusqlite::Row, from: usize) -> rusqlite::Result<Change> {
    Ok(Change {
        field: r.get(from)?,
        old_value: r.get(from + 1)?,
        new_value: r.get(from + 2)?,
        at: r.get(from + 3)?,
    })
}

/// Every issue's unseen changes, oldest first.
pub fn unseen_changes(conn: &Connection) -> AppResult<HashMap<String, Vec<Change>>> {
    let mut query = conn.prepare(
        "SELECT c.issue_id, c.field, c.old_value, c.new_value, c.changed_at FROM cache.issue_change c
         LEFT JOIN cache.issue_seen s ON s.issue_id = c.issue_id
         WHERE c.field != 'gone' AND (s.seen_at IS NULL OR c.changed_at > s.seen_at)
         ORDER BY c.changed_at, c.id",
    )?;
    let rows = query.query_map([], |r| Ok((r.get::<_, String>(0)?, change(r, 1)?)))?;
    let mut by_issue: HashMap<String, Vec<Change>> = HashMap::new();
    for row in rows {
        let (id, change) = row?;
        by_issue.entry(id).or_default().push(change);
    }
    Ok(by_issue)
}

/// One issue's changes after `after`, oldest first; "" returns them all.
pub fn changes_after(conn: &Connection, id: &str, after: &str) -> AppResult<Vec<Change>> {
    let mut query = conn.prepare(
        "SELECT field, old_value, new_value, changed_at FROM cache.issue_change
         WHERE issue_id = ?1 AND field != 'gone' AND changed_at > ?2
         ORDER BY changed_at, id",
    )?;
    let rows = query.query_map(params![id, after], |r| change(r, 0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Where new activity starts while the issue has unseen changes: the last look, or "" when never seen.
pub fn unseen_since(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    let unseen: bool = conn.query_row(
        &format!("SELECT EXISTS (SELECT 1 FROM cache.issue i WHERE i.id = ?1 AND {UNSEEN})"),
        [id],
        |r| r.get(0),
    )?;
    Ok(if unseen {
        Some(seen_at(conn, id)?.unwrap_or_default())
    } else {
        None
    })
}

/// When the owner last saw the issue; none when never.
pub fn seen_at(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT seen_at FROM cache.issue_seen WHERE issue_id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?)
}
