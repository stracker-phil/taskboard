//! The owner's own changes made in Jira itself aren't news. After a sync, the changelog of the issues it found
//! changed says who made each change; the connected account's are dropped. The fetched changelog is kept as the
//! issue's history.

use chrono::Utc;
use rusqlite::{Connection, params};

use super::source::Source;
use crate::changes::{catch_up, seen_at};
use crate::db::Db;
use crate::error::AppResult;
use crate::issues::store_history;
use crate::jira::dto::Changelog;
use crate::time::utc_text;

/// At most this many issues are checked per sync, the most recently updated first; the rest stay as recorded.
const CHECKED_ISSUES: usize = 50;

/// A recorded field the changelog can tell the author of.
enum Checked {
    /// A field by its changelog name.
    Field(&'static str),
    /// `updated`: any field since the owner last looked.
    Updated,
}

/// Recorded field (`issue_change.field`) to what the changelog says about it.
const CHECKED_FIELDS: &[(&str, Checked)] = &[
    ("status", Checked::Field("status")),
    ("assignee", Checked::Field("assignee")),
    ("priority", Checked::Field("priority")),
    ("due_date", Checked::Field("duedate")),
    ("updated", Checked::Updated),
];

fn checked(field: &str) -> Option<&'static Checked> {
    CHECKED_FIELDS
        .iter()
        .find(|(recorded, _)| *recorded == field)
        .map(|(_, checked)| checked)
}

/// The newest `cache.issue_change` row; a sync's changes come after it.
pub fn newest_change(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(rowid), 0) FROM cache.issue_change",
        [],
        |r| r.get(0),
    )?)
}

/// Best effort: without a connected account nothing is dropped; a failure keeps the changes as recorded.
pub async fn drop_own_changes(db: &Db, source: &dyn Source, me: Option<&str>, after: i64) {
    let Some(me) = me else {
        return;
    };
    if let Err(e) = check(db, source, me, after).await {
        log::info!("Own changes not checked: {e}");
    }
}

async fn check(db: &Db, source: &dyn Source, me: &str, after: i64) -> AppResult<()> {
    let ids = db.read(move |conn| changed_issues(conn, after)).await?;
    let source_name = source.kind().as_str();
    for id in ids {
        let entries = source.changelog(&id).await?;
        let me = me.to_string();
        db.write(move |tx, changes| {
            store_history(tx, changes, &id, &entries, source_name, Utc::now())?;
            if drop_mine(tx, &id, &entries, &me, after)? {
                changes.issue_changed(&id);
                catch_up(tx, changes, &id)?;
            }
            Ok(())
        })
        .await?;
    }
    Ok(())
}

/// The issues with a change after `after` that the changelog can tell the author of.
fn changed_issues(conn: &Connection, after: i64) -> AppResult<Vec<String>> {
    let fields: Vec<&str> = CHECKED_FIELDS.iter().map(|(field, _)| *field).collect();
    let mut query = conn.prepare(
        "SELECT i.id FROM cache.issue i
         WHERE i.gone = 0 AND EXISTS (SELECT 1 FROM cache.issue_change c
           WHERE c.issue_id = i.id AND c.rowid > ?1 AND c.field IN (SELECT value FROM json_each(?3)))
         ORDER BY i.updated_at DESC LIMIT ?2",
    )?;
    let ids = query.query_map(
        params![
            after,
            CHECKED_ISSUES as i64,
            serde_json::to_string(&fields)?
        ],
        |r| r.get(0),
    )?;
    Ok(ids.collect::<Result<_, _>>()?)
}

struct Entry {
    at: String,
    author: Option<String>,
    fields: Vec<String>,
}

fn entries_of(changelog: &[Changelog]) -> Vec<Entry> {
    changelog
        .iter()
        .filter_map(|entry| {
            Some(Entry {
                at: utc_text(&entry.created)?,
                author: entry.author.as_ref().map(|a| a.account_id.clone()),
                fields: entry
                    .items
                    .iter()
                    .map(|item| item.field_id.clone().unwrap_or_else(|| item.field.clone()))
                    .collect(),
            })
        })
        .collect()
}

struct Recorded {
    rowid: i64,
    field: String,
    changed_at: String,
}

fn recorded_after(conn: &Connection, id: &str, after: i64) -> AppResult<Vec<Recorded>> {
    let mut query = conn.prepare(
        "SELECT rowid, field, changed_at FROM cache.issue_change WHERE issue_id = ?1 AND rowid > ?2",
    )?;
    let rows = query.query_map(params![id, after], |r| {
        Ok(Recorded {
            rowid: r.get(0)?,
            field: r.get(1)?,
            changed_at: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Deletes the issue's changes after `after` that `me` made. Returns whether any went.
fn drop_mine(
    conn: &Connection,
    id: &str,
    changelog: &[Changelog],
    me: &str,
    after: i64,
) -> AppResult<bool> {
    let entries = entries_of(changelog);
    // Never seen: every entry is after the last look.
    let seen = seen_at(conn, id)?.unwrap_or_default();
    let mut dropped = false;
    for change in recorded_after(conn, id, after)? {
        let by_me = match checked(&change.field) {
            Some(Checked::Field(name)) => {
                last_change_by(&entries, name, &change.changed_at) == Some(me)
            }
            Some(Checked::Updated) => all_since_by(&entries, &seen, me),
            None => false,
        };
        if by_me {
            conn.execute(
                "DELETE FROM cache.issue_change WHERE rowid = ?1",
                [change.rowid],
            )?;
            dropped = true;
        }
    }
    Ok(dropped)
}

/// Who changed `field` last, up to `until`: the change that made the value the sync found.
fn last_change_by<'a>(entries: &'a [Entry], field: &str, until: &str) -> Option<&'a str> {
    entries
        .iter()
        .filter(|e| e.at.as_str() <= until && e.fields.iter().any(|f| f == field))
        .max_by(|a, b| a.at.cmp(&b.at))
        .and_then(|e| e.author.as_deref())
}

/// Every change after the owner last looked was `me`'s; none at all means the changelog can't tell.
fn all_since_by(entries: &[Entry], seen: &str, me: &str) -> bool {
    let mut since = entries.iter().filter(|e| e.at.as_str() > seen).peekable();
    since.peek().is_some() && since.all(|e| e.author.as_deref() == Some(me))
}
