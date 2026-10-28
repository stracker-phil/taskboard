//! Marking issues seen: by the owner (Mark seen, Mark all seen) or by leaving an issue they opened.

use rusqlite::{Connection, params};

use crate::db::{Changes, Db};
use crate::error::AppResult;

/// Seen at `now`, or at the issue's newest change or comment when Jira's clock runs ahead of this Mac's, so
/// nothing already shown stays unseen.
fn mark(conn: &Connection, changes: &mut Changes, ids: &[String], now: &str) -> AppResult<()> {
    let mut insert = conn.prepare(
        "INSERT INTO cache.issue_seen (issue_id, seen_at)
         SELECT i.id, MAX(?2,
           COALESCE((SELECT MAX(changed_at) FROM cache.issue_change WHERE issue_id = i.id), ''),
           COALESCE((SELECT MAX(created_at) FROM cache.comment WHERE issue_id = i.id), ''))
         FROM cache.issue i WHERE i.id = ?1
         ON CONFLICT (issue_id) DO UPDATE SET seen_at = MAX(seen_at, excluded.seen_at)",
    )?;
    for id in ids {
        if insert.execute(params![id, now])? > 0 {
            changes.issue_changed(id);
        }
    }
    Ok(())
}

pub async fn mark_seen(db: &Db, ids: Vec<String>, now: String) -> AppResult<()> {
    db.write(move |tx, changes| mark(tx, changes, &ids, &now))
        .await
}

/// Marks every issue `unseen` finds; it runs in the same transaction.
pub async fn mark_all_seen<F>(db: &Db, unseen: F, now: String) -> AppResult<()>
where
    F: FnOnce(&Connection) -> AppResult<Vec<String>> + Send + 'static,
{
    db.write(move |tx, changes| {
        let ids = unseen(tx)?;
        mark(tx, changes, &ids, &now)
    })
    .await
}
