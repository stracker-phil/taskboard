//! An issue's activity, newest first: its comments, its history (the tracker's changelog) and the field changes
//! the sync recorded after the newest history entry.

mod history;
mod recorded;
mod values;

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::changes::unseen_since;
use crate::db::Db;
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::status_style::Style;
use crate::views::rows::Person;
use values::Values;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ActivityItem {
    Comment {
        id: String,
        author: Option<Person>,
        /// Jira's rendered HTML, untrusted: the page sanitizes it.
        body_html: Option<String>,
        /// UTC, RFC 3339.
        at: String,
        edited: bool,
        /// After the owner last saw the issue, while it has unseen changes.
        fresh: bool,
    },
    Change {
        /// `status`, `assignee`, `reporter`, `due_date`, `priority`, `created`, else the tracker's name of the
        /// field (`Fix Version`, `Sprint`, `labels`, `description`).
        field: String,
        /// Both none for long text (a description): the change has no values to show.
        from: Option<ChangeValue>,
        to: Option<ChangeValue>,
        /// Who made the change; none when the sync recorded it.
        author: Option<Person>,
        at: String,
        fresh: bool,
    },
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ChangeValue {
    Status { name: String, style: Style },
    Person(Person),
    Text(String),
}

pub async fn issue_activity(db: &Db, viewer: Viewer, id: String) -> AppResult<Vec<ActivityItem>> {
    db.read(move |conn| activity(conn, &viewer, &id)).await
}

fn activity(conn: &Connection, viewer: &Viewer, id: &str) -> AppResult<Vec<ActivityItem>> {
    let values = Values::load(conn, id, viewer.me.as_deref())?;
    let history = history::items(conn, id, &values)?;
    let newest_history = history.iter().map(at).max().map(str::to_owned);
    let mut items = comments(conn, id, &values)?;
    items.extend(recorded::items(
        conn,
        id,
        &values,
        newest_history.as_deref(),
    )?);
    items.extend(history);
    items.sort_by(|a, b| at(b).cmp(at(a)));
    mark_fresh(conn, id, &mut items)?;
    Ok(items)
}

/// While the issue is unseen, the items after the owner's last look are fresh.
fn mark_fresh(conn: &Connection, id: &str, items: &mut [ActivityItem]) -> AppResult<()> {
    let Some(boundary) = unseen_since(conn, id)? else {
        return Ok(());
    };
    for item in items {
        let newer = at(item) > boundary.as_str();
        match item {
            ActivityItem::Comment { fresh, .. } | ActivityItem::Change { fresh, .. } => {
                *fresh = newer
            }
        }
    }
    Ok(())
}

fn at(item: &ActivityItem) -> &str {
    match item {
        ActivityItem::Comment { at, .. } | ActivityItem::Change { at, .. } => at,
    }
}

fn comments(conn: &Connection, id: &str, values: &Values) -> AppResult<Vec<ActivityItem>> {
    let mut query = conn.prepare(
        "SELECT id, author_id, body_html, created_at, updated_at FROM cache.comment WHERE issue_id = ?1",
    )?;
    let rows = query.query_map([id], |r| {
        let author_id: Option<String> = r.get(1)?;
        let created: String = r.get(3)?;
        let updated: String = r.get(4)?;
        Ok(ActivityItem::Comment {
            id: r.get(0)?,
            author: author_id.map(|a| values.person(&a, None)),
            body_html: r.get(2)?,
            edited: updated != created,
            at: created,
            fresh: false,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub(crate) fn status_names(conn: &Connection) -> AppResult<HashMap<String, (String, String)>> {
    let mut query = conn.prepare("SELECT id, name, category FROM cache.status")?;
    let rows = query.query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub(crate) fn user_names(conn: &Connection) -> AppResult<HashMap<String, String>> {
    let mut query = conn.prepare("SELECT account_id, name FROM cache.user")?;
    let rows = query.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A text value; blank is none.
fn text(value: Option<String>) -> Option<ChangeValue> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(ChangeValue::Text)
}
