//! Detail sync: an opened issue's comments, fetched in full when the cached ones may be behind.

use chrono::Utc;
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::db::Db;
use crate::error::AppResult;
use crate::jira::dto::Comment;
use crate::sync::map::{format_utc, html_to_text, utc};
use crate::sync::source::Source;

/// Fetches the issue's comments unless they were fetched in full after its last update. Returns whether it
/// fetched; the cache emits `issues_changed` for the issue when the comments or their authors changed.
pub async fn issue_sync_comments(db: &Db, source: &dyn Source, id: String) -> AppResult<bool> {
    let lookup = id.clone();
    let Some(key) = db.read(move |conn| stale_key(conn, &lookup)).await? else {
        return Ok(false);
    };
    let comments = source.comments(&key).await?;
    let now = format_utc(Utc::now());
    db.write(move |tx, changes| {
        let before = snapshot(tx, &id)?;
        let authors_changed = store(tx, &id, &comments, &now)?;
        if authors_changed || snapshot(tx, &id)? != before {
            changes.issue_changed(&id);
        }
        Ok(())
    })
    .await?;
    Ok(true)
}

/// The issue's key when its comments were never fetched in full or are older than its last update.
fn stale_key(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT key FROM cache.issue
             WHERE id = ?1 AND gone = 0 AND (comments_synced_at IS NULL OR comments_synced_at < updated_at)",
            [id],
            |r| r.get(0),
        )
        .optional()?)
}

/// The issue's stored comments, every column, in a fixed order.
fn snapshot(conn: &Connection, id: &str) -> AppResult<Vec<Vec<SqlValue>>> {
    const COLUMNS: usize = 7;
    let mut query = conn.prepare(
        "SELECT id, author_id, body_html, body_text, created_at, updated_at, body_adf
         FROM cache.comment WHERE issue_id = ?1 ORDER BY id",
    )?;
    let rows = query.query_map([id], |r| (0..COLUMNS).map(|i| r.get(i)).collect())?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Replaces the issue's comments. Returns whether an author's name or picture changed.
fn store(conn: &Connection, id: &str, comments: &[Comment], now: &str) -> AppResult<bool> {
    let mut authors_changed = false;
    conn.execute("DELETE FROM cache.comment WHERE issue_id = ?1", [id])?;
    let mut texts = Vec::new();
    for comment in comments {
        let stamp = |text: &str| utc(&Value::from(text));
        let (Some(created), Some(updated)) = (stamp(&comment.created), stamp(&comment.updated))
        else {
            continue;
        };
        let body_text = comment
            .rendered_body
            .as_deref()
            .map(html_to_text)
            .unwrap_or_default();
        if let Some(author) = &comment.author {
            authors_changed |= conn.execute(
                "INSERT INTO cache.user (account_id, name, avatar_url) VALUES (?1, ?2, ?3)
                 ON CONFLICT (account_id) DO UPDATE SET name = excluded.name, avatar_url = excluded.avatar_url
                 WHERE name IS NOT excluded.name OR avatar_url IS NOT excluded.avatar_url",
                params![author.account_id, author.display_name, author.avatar_url()],
            )? > 0;
        }
        conn.execute(
            "INSERT INTO cache.comment (id, issue_id, author_id, body_html, body_text, created_at, updated_at, body_adf)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                comment.id,
                id,
                comment.author.as_ref().map(|a| a.account_id.clone()),
                comment.rendered_body,
                body_text,
                created,
                updated,
                Some(&comment.body).filter(|b| b.is_object()).map(Value::to_string)
            ],
        )?;
        texts.push(body_text);
    }
    conn.execute(
        "UPDATE cache.issue SET comments_synced_at = ?2, comment_count = ?3 WHERE id = ?1",
        params![id, now, comments.len() as i64],
    )?;
    conn.execute(
        "UPDATE cache.issue_fts SET comments = ?2 WHERE rowid = ?1",
        params![id.parse::<i64>().unwrap_or_default(), texts.join("\n")],
    )?;
    Ok(authors_changed)
}
