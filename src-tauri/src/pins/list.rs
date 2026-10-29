//! The Pinned view: active pins, newest first, and the ones that ended in the last 7 days.

use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use super::duration::time_left;
use super::store::active;
use crate::db::Db;
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::status_style::{Style, Styles};
use crate::sync::map::format_utc;
use crate::views::rows::{self, Person};

/// How long an ended pin stays in Recently unpinned.
const RECENT_DAYS: i64 = 7;
/// Characters kept from a comment body before the ellipsis.
const EXCERPT_CHARS: usize = 90;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Pins {
    pub active: Vec<PinItem>,
    pub recently_unpinned: Vec<PinItem>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct PinItem {
    pub id: i64,
    pub target: Pinned,
    pub reason: Option<String>,
    /// "today", "2d left"; empty until unpinned and for ended pins.
    pub time_left: String,
    /// UTC; none until unpinned. The page refreshes when the first one passes.
    pub expires_at: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Pinned {
    Issue {
        id: String,
        key: String,
        summary: String,
        style: Style,
        /// Deleted in Jira or no longer visible; still reachable from here.
        gone: bool,
    },
    Comment {
        id: String,
        issue_id: String,
        issue_key: String,
        author: Option<Person>,
        excerpt: String,
    },
    List {
        id: i64,
        name: String,
        /// The Space it belongs to; none for a global list.
        space: Option<String>,
    },
}

struct Row {
    id: i64,
    kind: String,
    target: String,
    reason: Option<String>,
    expires_at: Option<String>,
}

pub async fn pins_list(db: &Db, viewer: Viewer) -> AppResult<Pins> {
    db.read(move |conn| {
        let now = format_utc(viewer.now);
        let since = format_utc(viewer.now - Duration::days(RECENT_DAYS));
        let style_map = Styles::load(conn)?;
        let active_rows = read(
            conn,
            &format!("{} ORDER BY p.created_at DESC, p.id DESC", active("?1")),
            &[&now],
        )?;
        let ended_rows = read(
            conn,
            "COALESCE(p.unpinned_at, p.expires_at) <= ?1 AND COALESCE(p.unpinned_at, p.expires_at) > ?2
             ORDER BY COALESCE(p.unpinned_at, p.expires_at) DESC, p.id DESC",
            &[&now, &since],
        )?;
        let mut active_items = resolve(conn, &viewer, &style_map, active_rows)?;
        for item in &mut active_items {
            let expires = item.expires_at.as_deref().and_then(|at| at.parse::<DateTime<Utc>>().ok());
            item.time_left = time_left(expires, viewer.now, viewer.tz());
        }
        Ok(Pins {
            active: active_items,
            recently_unpinned: resolve(conn, &viewer, &style_map, ended_rows)?,
        })
    })
    .await
}

/// The rows whose target still exists, without a time left.
fn resolve(
    conn: &Connection,
    viewer: &Viewer,
    style_map: &Styles,
    rows: Vec<Row>,
) -> AppResult<Vec<PinItem>> {
    let mut items = Vec::new();
    for row in rows {
        let found = match row.kind.as_str() {
            "issue" => issue_target(conn, style_map, &row.target)?,
            "comment" => comment_target(conn, viewer, &row.target)?,
            "filter_list" => list_target(conn, &row.target)?,
            _ => None,
        };
        if let Some(target) = found {
            items.push(PinItem {
                id: row.id,
                target,
                reason: row.reason,
                time_left: String::new(),
                expires_at: row.expires_at,
            });
        }
    }
    Ok(items)
}

fn read(
    conn: &Connection,
    condition: &str,
    params: &[&dyn rusqlite::ToSql],
) -> AppResult<Vec<Row>> {
    let mut query = conn.prepare(&format!(
        "SELECT p.id, p.target_kind, p.target_id, p.reason, p.expires_at FROM pin p WHERE {condition}"
    ))?;
    let rows = query.query_map(params, |r| {
        Ok(Row {
            id: r.get(0)?,
            kind: r.get(1)?,
            target: r.get(2)?,
            reason: r.get(3)?,
            expires_at: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

// The targets below are none when they no longer exist (a comment Jira deleted, a list the owner removed).

fn issue_target(conn: &Connection, style_map: &Styles, id: &str) -> AppResult<Option<Pinned>> {
    Ok(conn
        .query_row(
            "SELECT i.key, i.summary, i.status_id, COALESCE(st.name, ''), i.status_category, i.gone, i.project_id
             FROM cache.issue i LEFT JOIN cache.status st ON st.id = i.status_id WHERE i.id = ?1",
            [id],
            |r| {
                let status_id: String = r.get(2)?;
                let name: String = r.get(3)?;
                let category: String = r.get(4)?;
                Ok(Pinned::Issue {
                    id: id.to_string(),
                    key: r.get(0)?,
                    summary: r.get(1)?,
                    style: style_map.of(&r.get::<_, String>(6)?, &status_id, &category, &name),
                    gone: r.get::<_, i64>(5)? != 0,
                })
            },
        )
        .optional()?)
}

fn comment_target(conn: &Connection, viewer: &Viewer, id: &str) -> AppResult<Option<Pinned>> {
    Ok(conn
        .query_row(
            "SELECT c.issue_id, i.key, c.author_id, u.name, COALESCE(c.body_text, '')
             FROM cache.comment c JOIN cache.issue i ON i.id = c.issue_id
             LEFT JOIN cache.user u ON u.account_id = c.author_id WHERE c.id = ?1",
            [id],
            |r| {
                let author: Option<String> = r.get(2)?;
                let name: Option<String> = r.get(3)?;
                let body: String = r.get(4)?;
                Ok(Pinned::Comment {
                    id: id.to_string(),
                    issue_id: r.get(0)?,
                    issue_key: r.get(1)?,
                    author: author.map(|a| {
                        rows::person(&a, name.as_deref().unwrap_or(&a), viewer.me.as_deref())
                    }),
                    excerpt: excerpt(&body),
                })
            },
        )
        .optional()?)
}

fn list_target(conn: &Connection, id: &str) -> AppResult<Option<Pinned>> {
    Ok(conn
        .query_row(
            "SELECT l.id, l.name, COALESCE(s.name, p.name)
             FROM filter_list l
             LEFT JOIN filter_list sp ON sp.id = l.parent_id AND sp.kind = 'space'
             LEFT JOIN space s ON s.project_id = sp.project_id
             LEFT JOIN cache.space p ON p.id = sp.project_id
             WHERE CAST(l.id AS TEXT) = ?1",
            [id],
            |r| {
                Ok(Pinned::List {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    space: r.get(2)?,
                })
            },
        )
        .optional()?)
}

fn excerpt(body: &str) -> String {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= EXCERPT_CHARS {
        return flat;
    }
    let cut: String = flat.chars().take(EXCERPT_CHARS).collect();
    format!("{}…", cut.trim_end())
}
