//! An issue's web links (Jira's remote links: pull requests, documents): fetched when the issue is opened, at
//! most once per `FRESH_FOR`, cached in `cache.remote_link`, shown in the Issue view and the peek.

mod classify;

use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::db::Db;
use crate::error::AppResult;
use crate::jira::dto::RemoteLink;
use crate::sync::source::Source;

pub use classify::{Classified, WebLinkKind, classify};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct WebLink {
    pub id: String,
    pub url: String,
    /// The link's title, else its address.
    pub title: String,
    pub host: Option<String>,
    pub kind: WebLinkKind,
    /// `repo#481` for a pull request.
    pub reference: Option<String>,
    /// Only https links open; the rest show as text.
    pub openable: bool,
}

/// How long fetched links count as current: moving through issues doesn't ask the tracker for each again.
pub const FRESH_FOR: Duration = Duration::minutes(5);

/// The cached web links, in the tracker's order.
pub async fn issue_web_links(db: &Db, id: String) -> AppResult<Vec<WebLink>> {
    db.read(move |conn| cached(conn, &id)).await
}

/// Fetches the issue's web links and replaces the cached ones, unless they were fetched within `FRESH_FOR` of
/// `now`. Returns whether they changed; the cache emits `issues_changed` for the issue when they did. On a
/// failure the cached links stay.
pub async fn issue_sync_web_links(
    db: &Db,
    source: &dyn Source,
    id: String,
    now: DateTime<Utc>,
) -> AppResult<bool> {
    let lookup = id.clone();
    let Some(synced_at) = db.read(move |conn| last_synced(conn, &lookup)).await? else {
        return Ok(false);
    };
    if synced_at.is_some_and(|at| now - at < FRESH_FOR) {
        return Ok(false);
    }
    let links = match source.remote_links(&id).await {
        Ok(links) => links,
        Err(e) => {
            log::info!("Web links of issue {id} not fetched: {e}");
            return Err(e);
        }
    };
    let source_name = source.kind().as_str();
    db.write(move |tx, changes| {
        let changed = store(tx, &id, &links, source_name)?;
        tx.execute(
            "UPDATE cache.issue SET links_synced_at = ?2 WHERE id = ?1",
            params![id, now.to_rfc3339()],
        )?;
        if changed {
            changes.issue_changed(&id);
        }
        Ok(changed)
    })
    .await
}

/// None when the issue isn't cached (or is gone); else when its links were last fetched, if ever.
fn last_synced(conn: &Connection, id: &str) -> AppResult<Option<Option<DateTime<Utc>>>> {
    let at: Option<Option<String>> = conn
        .query_row(
            "SELECT links_synced_at FROM cache.issue WHERE id = ?1 AND gone = 0",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(at.map(|at| {
        at.and_then(|at| DateTime::parse_from_rfc3339(&at).ok())
            .map(|at| at.with_timezone(&Utc))
    }))
}

/// One stored row: what the tracker said, before it is classified for display.
#[derive(Debug, PartialEq)]
struct Row {
    id: String,
    url: String,
    title: Option<String>,
    application: Option<String>,
    relationship: Option<String>,
}

fn rows(conn: &Connection, id: &str) -> AppResult<Vec<Row>> {
    let mut stmt = conn.prepare(
        "SELECT id, url, title, application, relationship FROM cache.remote_link
         WHERE issue_id = ?1 ORDER BY position",
    )?;
    let rows = stmt
        .query_map([id], |r| {
            Ok(Row {
                id: r.get(0)?,
                url: r.get(1)?,
                title: r.get(2)?,
                application: r.get(3)?,
                relationship: r.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn cached(conn: &Connection, id: &str) -> AppResult<Vec<WebLink>> {
    Ok(rows(conn, id)?.into_iter().map(web_link).collect())
}

fn web_link(row: Row) -> WebLink {
    let Classified {
        kind,
        host,
        reference,
        openable,
    } = classify(&row.url);
    let title = row
        .title
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| row.url.clone());
    WebLink {
        id: row.id,
        url: row.url,
        title,
        host,
        kind,
        reference,
        openable,
    }
}

/// Replaces the issue's links; links without an address are left out. Returns whether anything changed.
fn store(conn: &Connection, id: &str, links: &[RemoteLink], source: &str) -> AppResult<bool> {
    let fresh: Vec<Row> = links
        .iter()
        .filter(|link| !link.object.url.trim().is_empty())
        .map(|link| Row {
            id: link.id.to_string(),
            url: link.object.url.clone(),
            title: link.object.title.clone(),
            application: link
                .application
                .as_ref()
                .and_then(|app| app.name.clone().or_else(|| app.kind.clone())),
            relationship: link.relationship.clone(),
        })
        .collect();
    if rows(conn, id)? == fresh {
        return Ok(false);
    }
    conn.execute("DELETE FROM cache.remote_link WHERE issue_id = ?1", [id])?;
    for (position, row) in fresh.iter().enumerate() {
        conn.execute(
            "INSERT OR REPLACE INTO cache.remote_link
               (issue_id, id, position, url, title, application, relationship, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                row.id,
                position as i64,
                row.url,
                row.title,
                row.application,
                row.relationship,
                source
            ],
        )?;
    }
    Ok(true)
}
