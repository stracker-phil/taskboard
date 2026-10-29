//! An issue's history: the tracker's changelog (who changed which field when), fetched when the issue is
//! opened, at most once per `fetched::FRESH_FOR`, cached in `cache.issue_history`, shown in the activity.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde_json::Value;

use super::fetched::{self, Detail};
use crate::db::Db;
use crate::error::AppResult;
use crate::jira::dto::Changelog;
use crate::sync::map::utc;
use crate::sync::source::Source;

/// Fetches the issue's changelog and replaces the cached history, unless it was fetched within `FRESH_FOR` of
/// `now`. Returns whether it changed; the cache emits `issues_changed` for the issue when it did. On a failure
/// the cached history stays.
pub async fn issue_sync_history(
    db: &Db,
    source: &dyn Source,
    id: String,
    now: DateTime<Utc>,
) -> AppResult<bool> {
    if !fetched::due(db, &id, Detail::History, now).await? {
        return Ok(false);
    }
    let entries = source
        .changelog(&id)
        .await
        .inspect_err(|e| log::info!("History of issue {id} not fetched: {e}"))?;
    let source_name = source.kind().as_str();
    db.write(move |tx, changes| {
        let changed = store(tx, &id, &entries, source_name)?;
        fetched::mark_fetched(tx, &id, Detail::History, now)?;
        if changed {
            changes.issue_changed(&id);
        }
        Ok(changed)
    })
    .await
}

/// One field's change, as stored.
#[derive(Debug, PartialEq)]
pub(super) struct HistoryRow {
    pub field: String,
    pub field_id: Option<String>,
    pub from_value: Option<String>,
    pub from_text: Option<String>,
    pub to_value: Option<String>,
    pub to_text: Option<String>,
    pub author_id: Option<String>,
    /// UTC, sortable as text like `issue_change.changed_at`.
    pub changed_at: String,
}

/// The issue's history, oldest first.
pub(super) fn rows(conn: &Connection, id: &str) -> AppResult<Vec<HistoryRow>> {
    let mut stmt = conn.prepare(
        "SELECT field, field_id, from_value, from_text, to_value, to_text, author_id, changed_at
         FROM cache.issue_history WHERE issue_id = ?1 ORDER BY position",
    )?;
    let rows = stmt
        .query_map([id], |r| {
            Ok(HistoryRow {
                field: r.get(0)?,
                field_id: r.get(1)?,
                from_value: r.get(2)?,
                from_text: r.get(3)?,
                to_value: r.get(4)?,
                to_text: r.get(5)?,
                author_id: r.get(6)?,
                changed_at: r.get(7)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Replaces the issue's history; entries with an unreadable time are left out. Returns whether it changed.
fn store(conn: &Connection, id: &str, entries: &[Changelog], source: &str) -> AppResult<bool> {
    let mut incoming = Vec::new();
    for entry in entries {
        let Some(changed_at) = utc(&Value::from(entry.created.as_str())) else {
            continue;
        };
        if let Some(author) = entry.author.as_ref().filter(|a| !a.display_name.is_empty()) {
            conn.execute(
                "INSERT INTO cache.user (account_id, name, avatar_url) VALUES (?1, ?2, ?3)
                 ON CONFLICT (account_id) DO UPDATE SET name = excluded.name, avatar_url = excluded.avatar_url",
                params![author.account_id, author.display_name, author.avatar_url()],
            )?;
        }
        incoming.extend(entry.items.iter().map(|item| HistoryRow {
            field: item.field.clone(),
            field_id: item.field_id.clone(),
            from_value: item.from.clone(),
            from_text: item.from_text.clone(),
            to_value: item.to.clone(),
            to_text: item.to_text.clone(),
            author_id: entry.author.as_ref().map(|a| a.account_id.clone()),
            changed_at: changed_at.clone(),
        }));
    }
    if rows(conn, id)? == incoming {
        return Ok(false);
    }
    conn.execute("DELETE FROM cache.issue_history WHERE issue_id = ?1", [id])?;
    for (position, row) in incoming.iter().enumerate() {
        conn.execute(
            "INSERT INTO cache.issue_history (issue_id, position, field, field_id, from_value, from_text,
               to_value, to_text, author_id, changed_at, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                position as i64,
                row.field,
                row.field_id,
                row.from_value,
                row.from_text,
                row.to_value,
                row.to_text,
                row.author_id,
                row.changed_at,
                source
            ],
        )?;
    }
    Ok(true)
}
