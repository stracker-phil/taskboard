//! Who made the changes a sync recorded without an author. After a sync, the changelog of the issues it found
//! changed names the author of each change; the connected account's are dropped, and the rest are judged again
//! (an automation's may not count). The fetched changelog is kept as the issue's history.

use chrono::Utc;
use rusqlite::{Connection, params};

use super::authors::{CHECKED_FIELDS, entries_of, from_changelog};
use super::source::Source;
use crate::changes::{catch_up, seen_at};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::issues::store_history;
use crate::jira::dto::Changelog;
use crate::notify::rules::settle;
use crate::store::settings::Notifications;

/// At most this many issues are checked per sync, the most recently updated first; the rest stay unknown.
const CHECKED_ISSUES: usize = 50;

/// The newest `cache.issue_change` row; a sync's changes come after it.
pub fn newest_change(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(rowid), 0) FROM cache.issue_change",
        [],
        |r| r.get(0),
    )?)
}

/// Best effort: a failure keeps the changes as recorded, with their authors unknown.
pub async fn attribute_changes(
    db: &Db,
    source: &dyn Source,
    notifications: &Notifications,
    me: Option<&str>,
    after: i64,
) {
    if let Err(e) = check(db, source, notifications, me, after).await {
        log::info!("Authors of changes not checked: {e}");
    }
}

async fn check(
    db: &Db,
    source: &dyn Source,
    notifications: &Notifications,
    me: Option<&str>,
    after: i64,
) -> AppResult<()> {
    let ids = db
        .read(move |conn| unattributed_issues(conn, after))
        .await?;
    let source_name = source.kind().as_str();
    for id in ids {
        let entries = match source.changelog(&id).await {
            Ok(entries) => entries,
            // Deleted in Jira since the search: the reconcile marks it gone; the others still get their authors.
            Err(AppError::NotFound(_)) => continue,
            Err(e) => return Err(e),
        };
        let me = me.map(String::from);
        let notifications = notifications.clone();
        db.write(move |tx, changes| {
            store_history(tx, changes, &id, &entries, source_name, Utc::now())?;
            if !attribute(tx, &id, &entries, after)? {
                return Ok(());
            }
            changes.issue_changed(&id);
            let dropped = match me.as_deref() {
                Some(me) => drop_mine(tx, &id, me, after)?,
                None => false,
            };
            // Judged before catching up, which asks what still counts.
            settle(tx, changes, &notifications, me.as_deref(), false)?;
            if dropped {
                catch_up(tx, changes, &id)?;
            }
            Ok(())
        })
        .await?;
    }
    Ok(())
}

/// The issues with a change after `after` whose author is unknown and that the changelog can tell.
fn unattributed_issues(conn: &Connection, after: i64) -> AppResult<Vec<String>> {
    let mut query = conn.prepare(
        "SELECT i.id FROM cache.issue i
         WHERE i.gone = 0 AND EXISTS (SELECT 1 FROM cache.issue_change c
           WHERE c.issue_id = i.id AND c.rowid > ?1 AND c.author_id IS NULL
             AND c.field IN (SELECT value FROM json_each(?3)))
         ORDER BY i.updated_at DESC LIMIT ?2",
    )?;
    let ids = query.query_map(
        params![
            after,
            CHECKED_ISSUES as i64,
            serde_json::to_string(CHECKED_FIELDS)?
        ],
        |r| r.get(0),
    )?;
    Ok(ids.collect::<Result<_, _>>()?)
}

struct Unattributed {
    rowid: i64,
    field: String,
    old_value: Option<String>,
    changed_at: String,
}

fn unattributed_after(conn: &Connection, id: &str, after: i64) -> AppResult<Vec<Unattributed>> {
    let mut query = conn.prepare(
        "SELECT rowid, field, old_value, changed_at FROM cache.issue_change
         WHERE issue_id = ?1 AND rowid > ?2 AND author_id IS NULL",
    )?;
    let rows = query.query_map(params![id, after], |r| {
        Ok(Unattributed {
            rowid: r.get(0)?,
            field: r.get(1)?,
            old_value: r.get(2)?,
            changed_at: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Names the author of the issue's changes after `after` that the changelog tells; they are judged anew.
/// Returns whether any got one.
fn attribute(conn: &Connection, id: &str, changelog: &[Changelog], after: i64) -> AppResult<bool> {
    let entries = entries_of(changelog);
    // Never seen: every entry is after the last look.
    let seen = seen_at(conn, id)?.unwrap_or_default();
    let mut update = conn.prepare_cached(
        "UPDATE cache.issue_change SET author_id = ?2, quiet = NULL WHERE rowid = ?1",
    )?;
    let mut attributed = false;
    for change in unattributed_after(conn, id, after)? {
        let author = from_changelog(
            &entries,
            &change.field,
            change.old_value.as_deref(),
            &change.changed_at,
            &seen,
        );
        if let Some(author) = author {
            update.execute(params![change.rowid, author])?;
            attributed = true;
        }
    }
    Ok(attributed)
}

/// Deletes the issue's changes after `after` that `me` made. Returns whether any went.
fn drop_mine(conn: &Connection, id: &str, me: &str, after: i64) -> AppResult<bool> {
    let dropped = conn.execute(
        "DELETE FROM cache.issue_change WHERE issue_id = ?1 AND rowid > ?2 AND author_id = ?3",
        params![id, after, me],
    )?;
    Ok(dropped > 0)
}
