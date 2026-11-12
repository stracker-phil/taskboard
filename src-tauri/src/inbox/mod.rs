//! The inbox: every unread issue in one list (ADR 0059). Opening it takes a snapshot, so issues read while it is
//! open stay listed, dimmed, until it closes; the next open lists only what is unread then.

pub mod commands;

use std::collections::HashMap;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::changes::{Names, UNSEEN, facts, is_unseen, sentence, unseen_changes};
use crate::error::AppResult;
use crate::filter::TRACKED_ISSUES;
use crate::issues::{field_names, status_names, user_names};
use crate::status_style::{Style, Styles};

/// One unread issue, as the inbox lists it.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct InboxRow {
    pub id: String,
    pub key: String,
    pub summary: String,
    pub project_id: String,
    pub style: Style,
    pub status_name: String,
    /// What changed, when the inbox opened: "Moved to Review, 1 new comment."
    pub note: String,
    /// The newest unseen change (UTC); the issue's `updated` when it is only marked unread.
    pub changed_at: String,
    /// Still unread; a row read while the inbox is open stays, read.
    pub unread: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Inbox {
    /// Newest change first, in the order of the snapshot.
    pub rows: Vec<InboxRow>,
    /// How many of them are unread.
    pub unread: u32,
}

/// The open inbox's issues with what each note said when it opened.
#[derive(Default)]
pub struct InboxSession(Mutex<Option<Vec<Snapshot>>>);

#[derive(Clone)]
pub struct Snapshot {
    id: String,
    note: String,
    changed_at: String,
}

impl InboxSession {
    fn set(&self, snapshot: Option<Vec<Snapshot>>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = snapshot;
    }

    fn get(&self) -> Option<Vec<Snapshot>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// The unread tracked issues, newest change first, with their notes.
pub fn snapshot(conn: &Connection, me: Option<&str>) -> AppResult<Vec<Snapshot>> {
    let mut query = conn.prepare(&format!(
        "SELECT i.id, COALESCE((SELECT MAX(c.changed_at) FROM cache.issue_change c
             LEFT JOIN issue_seen s ON s.issue_id = c.issue_id
             WHERE c.issue_id = i.id AND c.field != 'gone' AND COALESCE(c.quiet, 0) = 0
               AND (s.seen_at IS NULL OR c.changed_at > s.seen_at)),
           i.updated_at) AS at
         FROM cache.issue i WHERE {TRACKED_ISSUES} AND {UNSEEN}
         ORDER BY at DESC, i.id"
    ))?;
    let found: Vec<(String, String)> = query
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let ids: Vec<&str> = found.iter().map(|(id, _)| id.as_str()).collect();
    let mut notes = notes(conn, &ids, me)?;
    Ok(found
        .into_iter()
        .map(|(id, changed_at)| Snapshot {
            note: notes
                .remove(&id)
                .unwrap_or_else(|| "Marked unread.".to_string()),
            id,
            changed_at,
        })
        .collect())
}

/// What changed for each issue with unseen changes, as a sentence.
fn notes(conn: &Connection, ids: &[&str], me: Option<&str>) -> AppResult<HashMap<String, String>> {
    let statuses: HashMap<String, String> = status_names(conn)?
        .into_iter()
        .map(|(id, (name, _))| (id, name))
        .collect();
    let people = user_names(conn)?;
    let fields = field_names(conn)?;
    let names = Names {
        me,
        statuses: &statuses,
        people: &people,
        fields: &fields,
    };
    Ok(unseen_changes(conn, ids)?
        .into_iter()
        .filter_map(|(id, changes)| Some((id, capitalized(&sentence(&facts(&changes), &names)?))))
        .collect())
}

/// The snapshot's issues as they are now; issues gone from the cache since are left out.
pub fn rows(conn: &Connection, snapshot: &[Snapshot]) -> AppResult<Inbox> {
    let styles = Styles::load(conn)?;
    let mut query = conn.prepare_cached(
        "SELECT i.key, i.summary, i.project_id, i.status_id, i.status_category, COALESCE(s.name, '')
         FROM cache.issue i LEFT JOIN cache.status s ON s.id = i.status_id WHERE i.id = ?1 AND i.gone = 0",
    )?;
    let mut rows = Vec::with_capacity(snapshot.len());
    for snap in snapshot {
        let found = query
            .query_row([&snap.id], |r| {
                Ok(Current {
                    key: r.get(0)?,
                    summary: r.get(1)?,
                    project_id: r.get(2)?,
                    status_id: r.get(3)?,
                    category: r.get(4)?,
                    status_name: r.get(5)?,
                })
            })
            .optional()?;
        let Some(now) = found else {
            continue;
        };
        rows.push(InboxRow {
            style: styles.of(
                &now.project_id,
                &now.status_id,
                &now.category,
                &now.status_name,
            ),
            unread: is_unseen(conn, &snap.id)?,
            id: snap.id.clone(),
            key: now.key,
            summary: now.summary,
            project_id: now.project_id,
            status_name: now.status_name,
            note: snap.note.clone(),
            changed_at: snap.changed_at.clone(),
        });
    }
    let unread = rows.iter().filter(|row| row.unread).count() as u32;
    Ok(Inbox { rows, unread })
}

/// An issue's row as the cache holds it now.
struct Current {
    key: String,
    summary: String,
    project_id: String,
    status_id: String,
    category: String,
    status_name: String,
}

/// How many tracked issues are unread: the bell's badge.
pub fn count(conn: &Connection) -> AppResult<u32> {
    Ok(conn.query_row(
        &format!("SELECT COUNT(*) FROM cache.issue i WHERE {TRACKED_ISSUES} AND {UNSEEN}"),
        [],
        |r| r.get(0),
    )?)
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
