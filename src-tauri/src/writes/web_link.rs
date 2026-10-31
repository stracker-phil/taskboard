//! Adding a web link (a pull request, a document) to an issue.

use rusqlite::params;

use super::{WriteOutcome, ensure_allowed, issue_key};
use crate::db::Db;
use crate::error::AppResult;
use crate::issues::classify;
use crate::jira::gateway::{LinkTitle, WebUrl};
use crate::store::settings::Settings;
use crate::sync::source::Source;

/// Jira's limit for a link's title.
const TITLE_MAX: usize = 255;

/// Adds the link to the issue in Jira and to the cached links. Without a title it takes the pull request's reference
/// (`repo#481`), else the address.
pub async fn add_web_link(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
    url: String,
    title: Option<String>,
) -> AppResult<WriteOutcome> {
    ensure_allowed(settings)?;
    let url = WebUrl::parse(&url)?.as_str().to_string();
    let title = match title.filter(|t| !t.trim().is_empty()) {
        Some(title) => LinkTitle::parse(&title)?.as_str().to_string(),
        None => default_title(&url),
    };
    let id = issue_id.clone();
    let key = db.read(move |conn| issue_key(conn, &id)).await?;
    let link_id = source.add_remote_link(&issue_id, &url, &title).await?;
    let source_name = source.kind().as_str();
    db.write(move |tx, changes| {
        tx.execute(
            "INSERT INTO cache.remote_link (issue_id, id, position, url, title, source)
             VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM cache.remote_link WHERE issue_id = ?1), ?3, ?4, ?5)
             ON CONFLICT (issue_id, id) DO NOTHING",
            params![issue_id, link_id, url, title, source_name],
        )?;
        changes.detail_changed(&issue_id);
        Ok(())
    })
    .await?;
    Ok(WriteOutcome {
        done: format!("Added the link to {key}"),
        reverse: None,
    })
}

fn default_title(url: &str) -> String {
    let title = classify(url).reference.unwrap_or_else(|| url.to_string());
    title.chars().take(TITLE_MAX).collect()
}
