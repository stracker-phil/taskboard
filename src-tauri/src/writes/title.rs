//! Renaming an issue.

use chrono::Utc;
use rusqlite::Connection;

use super::{Begun, Change, Field, Reverse, WriteOutcome, issue_key, pending, send};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::gateway::{IssueTitle, TITLE_MAX};
use crate::sync::source::Source;
use crate::time::format_utc;

pub(super) async fn set_title(
    db: &Db,
    source: &dyn Source,
    issue_id: String,
    text: String,
) -> AppResult<WriteOutcome> {
    let title = text.trim().to_string();
    if IssueTitle::parse(&title).is_err() {
        return Err(AppError::InvalidInput(format!(
            "A title is one line of 1 to {TITLE_MAX} characters"
        )));
    }
    let id = issue_id.clone();
    let target = title.clone();
    let now = format_utc(Utc::now());
    let begun = db
        .write(move |tx, changes| {
            let begun = begin(tx, &id, &target, &now)?;
            if matches!(begun, Begun::Sent(_)) {
                changes.issue_changed(&id);
            }
            Ok(begun)
        })
        .await?;
    let change = match begun {
        Begun::AlreadyThere(done) => {
            return Ok(WriteOutcome {
                done,
                reverse: None,
            });
        }
        Begun::Sent(change) => change,
    };
    let write = source.set_title(&issue_id, &title);
    let sent = format!("Title ({} characters)", title.chars().count());
    send(db, source, &issue_id, Field::Title, sent, write).await?;
    Ok(change.outcome())
}

struct TitleChange {
    key: String,
    before: Option<String>,
}

impl TitleChange {
    fn outcome(self) -> WriteOutcome {
        WriteOutcome {
            done: format!("Renamed {}", self.key),
            reverse: self.before.map(|before| Reverse {
                label: format!("Change back to “{before}”"),
                change: Change::Title { text: before },
            }),
        }
    }
}

fn begin(conn: &Connection, id: &str, title: &str, now: &str) -> AppResult<Begun<TitleChange>> {
    let key = issue_key(conn, id)?;
    let current: String = conn.query_row(
        "SELECT summary FROM cache.issue WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?;
    if current == title {
        return Ok(Begun::AlreadyThere(format!("{key} already has this title")));
    }
    let before = pending::begin(conn, id, Field::Title, Some(title), now)?;
    Ok(Begun::Sent(TitleChange { key, before }))
}
