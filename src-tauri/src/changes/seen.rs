//! Marking issues read (seen) by the owner or after the shown issue's delay, and unread again.

use chrono::{DateTime, Duration};
use rusqlite::{Connection, params};

use super::UNSEEN;
use crate::db::{Changes, Db};
use crate::error::{AppError, AppResult};
use crate::sync::map::format_utc;

/// Seen at `now`, or at the issue's newest change or comment when Jira's clock runs ahead of this Mac's, so
/// nothing already shown stays unseen. Only an issue that was unseen counts as changed: reading a read issue
/// again shows nothing new.
fn mark(conn: &Connection, changes: &mut Changes, ids: &[String], now: &str) -> AppResult<()> {
    let mut insert = conn.prepare(
        "INSERT INTO issue_seen (issue_id, seen_at)
         SELECT i.id, MAX(?2,
           COALESCE((SELECT MAX(changed_at) FROM cache.issue_change WHERE issue_id = i.id), ''),
           COALESCE((SELECT MAX(created_at) FROM cache.comment WHERE issue_id = i.id), ''))
         FROM cache.issue i WHERE i.id = ?1
         ON CONFLICT (issue_id) DO UPDATE SET seen_at = MAX(seen_at, excluded.seen_at), unread = 0",
    )?;
    let mut unseen = conn.prepare(&format!(
        "SELECT EXISTS (SELECT 1 FROM cache.issue i WHERE i.id = ?1 AND {UNSEEN})"
    ))?;
    for id in ids {
        let was_unseen: bool = unseen.query_row([id], |r| r.get(0))?;
        insert.execute(params![id, now])?;
        if was_unseen {
            changes.issue_changed(id);
        }
    }
    Ok(())
}

pub async fn mark_seen(db: &Db, ids: Vec<String>, now: String) -> AppResult<()> {
    db.write(move |tx, changes| mark(tx, changes, &ids, &now))
        .await
}

/// Marks every issue `unseen` finds; it runs in the same transaction. Returns their ids.
pub async fn mark_all_seen<F>(db: &Db, unseen: F, now: String) -> AppResult<Vec<String>>
where
    F: FnOnce(&Connection) -> AppResult<Vec<String>> + Send + 'static,
{
    db.write(move |tx, changes| {
        let ids = unseen(tx)?;
        mark(tx, changes, &ids, &now)?;
        Ok(ids)
    })
    .await
}

/// Unread again. With `from` (an activity item's time, RFC 3339) that item and everything newer count as new;
/// without, the issue is unread and its activity keeps its state.
pub async fn mark_unread(db: &Db, id: String, from: Option<String>) -> AppResult<()> {
    let before = match from {
        Some(at) => Some(just_before(&at)?),
        None => None,
    };
    db.write(move |tx, changes| {
        let marked = tx.execute(
            "INSERT INTO issue_seen (issue_id, seen_at, unread)
             SELECT id, COALESCE(?2, ''), 1 FROM cache.issue WHERE id = ?1
             ON CONFLICT (issue_id) DO UPDATE SET unread = 1,
               seen_at = CASE WHEN ?2 IS NULL THEN seen_at ELSE MIN(seen_at, ?2) END",
            params![id, before],
        )?;
        if marked > 0 {
            changes.issue_changed(&id);
        }
        Ok(())
    })
    .await
}

fn just_before(at: &str) -> AppResult<String> {
    let at = DateTime::parse_from_rfc3339(at)
        .map_err(|e| AppError::InvalidInput(format!("“{at}” isn't a time: {e}")))?;
    Ok(format_utc(at.to_utc() - Duration::milliseconds(1)))
}
