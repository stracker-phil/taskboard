//! An issue's activity: its comments and the field changes the sync recorded, newest first.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::changes::unseen_since;
use crate::db::Db;
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::status_style::{Style, Styles};
use crate::views::rows::{self, Person};

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
        /// Jira's rendered HTML, untrusted: the page sanitises it.
        body_html: Option<String>,
        /// UTC, RFC 3339.
        at: String,
        edited: bool,
        /// After the owner last saw the issue, while it has unseen changes.
        fresh: bool,
    },
    Change {
        /// `status`, `assignee`, `due_date`, `priority` or `created`.
        field: String,
        from: Option<ChangeValue>,
        to: Option<ChangeValue>,
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
    let me = viewer.me.clone();
    let mut items = comments(conn, id, me.as_deref())?;
    items.extend(changes(conn, id, me.as_deref())?);
    items.sort_by(|a, b| at(b).cmp(at(a)));
    if let Some(boundary) = unseen_since(conn, id)? {
        for item in &mut items {
            let newer = at(item) > boundary.as_str();
            match item {
                ActivityItem::Comment { fresh, .. } | ActivityItem::Change { fresh, .. } => {
                    *fresh = newer
                }
            }
        }
    }
    Ok(items)
}

fn at(item: &ActivityItem) -> &str {
    match item {
        ActivityItem::Comment { at, .. } | ActivityItem::Change { at, .. } => at,
    }
}

fn comments(conn: &Connection, id: &str, me: Option<&str>) -> AppResult<Vec<ActivityItem>> {
    let mut query = conn.prepare(
        "SELECT c.id, c.author_id, u.name, c.body_html, c.created_at, c.updated_at
         FROM cache.comment c LEFT JOIN cache.user u ON u.account_id = c.author_id
         WHERE c.issue_id = ?1",
    )?;
    let rows = query.query_map([id], |r| {
        let author_id: Option<String> = r.get(1)?;
        let name: Option<String> = r.get(2)?;
        let created: String = r.get(4)?;
        let updated: String = r.get(5)?;
        Ok(ActivityItem::Comment {
            id: r.get(0)?,
            author: author_id.map(|a| rows::person(&a, name.as_deref().unwrap_or(&a), me)),
            body_html: r.get(3)?,
            edited: updated != created,
            at: created,
            fresh: false,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// `comment_added` is left out: the comment itself is in the feed.
fn changes(conn: &Connection, id: &str, me: Option<&str>) -> AppResult<Vec<ActivityItem>> {
    let statuses = status_names(conn)?;
    let styles = Styles::load(conn)?;
    let project_id: String = conn.query_row(
        "SELECT project_id FROM cache.issue WHERE id = ?1",
        [id],
        |r| r.get(0),
    )?;
    let people = user_names(conn)?;
    let value = |field: &str, raw: Option<String>| -> Option<ChangeValue> {
        let raw = raw?;
        Some(match field {
            "status" => {
                let (name, category) = statuses
                    .get(&raw)
                    .cloned()
                    .unwrap_or((raw.clone(), String::new()));
                let style = styles.of(&project_id, &raw, &category, &name);
                ChangeValue::Status { name, style }
            }
            "assignee" => ChangeValue::Person(rows::person(
                &raw,
                people.get(&raw).map_or(raw.as_str(), String::as_str),
                me,
            )),
            _ => ChangeValue::Text(raw),
        })
    };
    let mut query = conn.prepare(
        "SELECT field, old_value, new_value, changed_at FROM cache.issue_change
         WHERE issue_id = ?1 AND field != 'comment_added'",
    )?;
    let rows = query.query_map([id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (field, old, new, at) = row?;
        items.push(ActivityItem::Change {
            from: value(&field, old),
            to: value(&field, new),
            field,
            at,
            fresh: false,
        });
    }
    Ok(items)
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
