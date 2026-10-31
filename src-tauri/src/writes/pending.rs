//! `cache.pending_write`: a change sent to the tracker but not settled. The issue row shows the sent value at
//! once; the sync keeps it there (`overlay`) until the write settles or fails.

use rusqlite::{Connection, OptionalExtension, params};

use super::Field;
use crate::error::{AppError, AppResult};
use crate::sync::map::IssueRow;

/// Shows `value` in the issue row and records the write; returns the value the row showed before. Refused while
/// another write to the same field of the issue is on its way.
pub fn begin(
    conn: &Connection,
    issue_id: &str,
    field: Field,
    value: Option<&str>,
    now: &str,
) -> AppResult<Option<String>> {
    let shown = shown_value(conn, issue_id, field)?;
    let inserted = conn.execute(
        "INSERT INTO cache.pending_write (issue_id, field, value, base, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (issue_id, field) DO NOTHING",
        params![issue_id, field.as_str(), value, shown, now],
    )?;
    if inserted == 0 {
        return Err(AppError::InvalidInput(
            "An earlier change to this issue is still on its way to Jira".into(),
        ));
    }
    set_value(conn, issue_id, field, value)?;
    Ok(shown)
}

/// The write failed: the row shows what the tracker last had again.
pub fn roll_back(conn: &Connection, issue_id: &str, field: Field) -> AppResult<()> {
    let base: Option<Option<String>> = conn
        .query_row(
            "SELECT base FROM cache.pending_write WHERE issue_id = ?1 AND field = ?2",
            params![issue_id, field.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(base) = base {
        set_value(conn, issue_id, field, base.as_deref())?;
    }
    settle(conn, issue_id, field)
}

/// The write reached the tracker; the sync may change the field again.
pub fn settle(conn: &Connection, issue_id: &str, field: Field) -> AppResult<()> {
    conn.execute(
        "DELETE FROM cache.pending_write WHERE issue_id = ?1 AND field = ?2",
        params![issue_id, field.as_str()],
    )?;
    Ok(())
}

/// Writes interrupted by a quit can't settle any more; the issue keeps the value it shows.
pub fn forget_all(conn: &Connection) -> AppResult<usize> {
    Ok(conn.execute("DELETE FROM cache.pending_write", [])?)
}

/// Puts the pending values over a row the sync is about to store, and keeps the row's own as the base.
pub fn overlay(conn: &Connection, row: &mut IssueRow) -> AppResult<()> {
    let mut query =
        conn.prepare_cached("SELECT field, value FROM cache.pending_write WHERE issue_id = ?1")?;
    let pending = query
        .query_map([&row.id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (field, value) in pending {
        let Some(field) = Field::parse(&field) else {
            continue;
        };
        let fetched = match (field, value) {
            (Field::Assignee, value) => std::mem::replace(&mut row.assignee_id, value),
            (Field::Status, Some(status_id)) => {
                if let Some(category) = status_category(conn, &status_id)? {
                    row.status_category = category;
                }
                Some(std::mem::replace(&mut row.status_id, status_id))
            }
            (Field::Status, None) => continue,
            (Field::Priority, value) => std::mem::replace(&mut row.priority, value),
        };
        conn.execute(
            "UPDATE cache.pending_write SET base = ?3 WHERE issue_id = ?1 AND field = ?2",
            params![row.id, field.as_str(), fetched],
        )?;
    }
    Ok(())
}

fn shown_value(conn: &Connection, issue_id: &str, field: Field) -> AppResult<Option<String>> {
    let column = match field {
        Field::Assignee => "assignee_id",
        Field::Status => "status_id",
        Field::Priority => "priority",
    };
    conn.query_row(
        &format!("SELECT {column} FROM cache.issue WHERE id = ?1 AND gone = 0"),
        [issue_id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound("The issue isn't in the cache".into()))
}

fn set_value(
    conn: &Connection,
    issue_id: &str,
    field: Field,
    value: Option<&str>,
) -> AppResult<()> {
    match field {
        Field::Assignee => conn.execute(
            "UPDATE cache.issue SET assignee_id = ?2 WHERE id = ?1",
            params![issue_id, value],
        )?,
        Field::Priority => conn.execute(
            "UPDATE cache.issue SET priority = ?2 WHERE id = ?1",
            params![issue_id, value],
        )?,
        Field::Status => conn.execute(
            "UPDATE cache.issue SET status_id = ?2,
               status_category = COALESCE((SELECT category FROM cache.status WHERE id = ?2), status_category)
             WHERE id = ?1",
            params![issue_id, value],
        )?,
    };
    Ok(())
}

/// `new`, `indeterminate` or `done`, from the cached statuses.
fn status_category(conn: &Connection, status_id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT category FROM cache.status WHERE id = ?1",
            [status_id],
            |row| row.get(0),
        )
        .optional()?)
}
