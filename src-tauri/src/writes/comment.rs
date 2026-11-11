//! Adding a comment. It shows in the activity at once as a `pending:` row that the comment syncs keep, then
//! Jira's comment takes its place.

use chrono::Utc;
use rusqlite::{Connection, params};

use super::{Me, WriteOutcome, ensure_allowed, issue_key, issue_project, me_of, remember_me};
use crate::adf::{self, Media, Mentionable};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::issues::store_comment;
use crate::jira::capability::Capability;
use crate::jira::dto::Comment;
use crate::store::settings::Settings;
use crate::sync::refetch_issue;
use crate::sync::source::Source;
use crate::text::html_to_text;
use crate::time::format_utc;
use crate::views::rows::Person;

/// Jira's comment length cap, in characters.
pub const MAX_COMMENT_CHARS: usize = 32_767;

pub async fn add_comment(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
    text: String,
) -> AppResult<WriteOutcome> {
    ensure_allowed(settings, Capability::Comment)?;
    let me = me_of(settings)?;
    let text = checked_text(&text)?;
    let people: Vec<Mentionable> = mentionable(db, issue_id.clone(), Some(me.clone()))
        .await?
        .into_iter()
        .map(Mentionable::from)
        .collect();
    let body = adf::from_markdown(&text, &people);
    let change = format!("Comment ({} characters)", text.chars().count());
    let html = adf::to_html(&body, &Media::default()).unwrap_or_default();
    let text = html_to_text(&html);
    let shown = Shown {
        issue_id: issue_id.clone(),
        pending_id: format!("pending:{}", fastrand::u64(..)),
        html,
        adf: body.to_string(),
        text,
        at: format_utc(Utc::now()),
    };
    let pending_id = shown.pending_id.clone();
    let key = db
        .write(move |tx, changes| {
            let key = issue_key(tx, &shown.issue_id)?;
            remember_me(tx, &me)?;
            show(tx, &me, &shown)?;
            changes.issue_changed(&shown.issue_id);
            Ok(key)
        })
        .await?;
    let answer = source.add_comment(&issue_id, body).await;
    super::sent::record(db, &issue_id, change, answer.as_ref().err()).await;
    match answer {
        Ok(stored) => settle(db, source, issue_id, pending_id, stored).await?,
        Err(e) => {
            log::info!("Jira refused a comment on issue {issue_id}: {e}");
            db.write(move |tx, changes| {
                withdraw(tx, &issue_id, &pending_id)?;
                changes.issue_changed(&issue_id);
                Ok(())
            })
            .await?;
            return Err(e);
        }
    }
    Ok(WriteOutcome {
        done: format!("Commented on {key}"),
        reverse: None,
    })
}

impl From<Person> for Mentionable {
    fn from(person: Person) -> Self {
        Mentionable {
            account_id: person.account_id,
            name: person.name,
        }
    }
}

/// Whom a comment on the issue may mention: the issue's Space's team, without me.
pub async fn mentionable(db: &Db, issue_id: String, me: Option<Me>) -> AppResult<Vec<Person>> {
    db.read(move |conn| {
        let project = issue_project(conn, &issue_id)?;
        let mut people = crate::team::active(conn, &[project], None)?;
        people.retain(|p| me.as_ref().is_none_or(|me| me.account_id != p.account_id));
        Ok(people)
    })
    .await
}

fn checked_text(text: &str) -> AppResult<String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::InvalidInput("Write something first".into()));
    }
    if text.chars().count() > MAX_COMMENT_CHARS {
        return Err(AppError::InvalidInput(format!(
            "Jira takes comments of up to {MAX_COMMENT_CHARS} characters"
        )));
    }
    Ok(text.to_string())
}

/// Jira's comment takes the pending one's place, with the issue fetched again in the same transaction.
async fn settle(
    db: &Db,
    source: &dyn Source,
    issue_id: String,
    pending_id: String,
    stored: Comment,
) -> AppResult<()> {
    let (id, pending) = (issue_id.clone(), pending_id.clone());
    let refetched = refetch_issue(db, source, &issue_id, move |tx| {
        remove_pending(tx, &pending)?;
        store_comment(tx, &id, &stored)?;
        Ok(())
    })
    .await;
    if let Err(e) = refetched {
        // The comment is in Jira; the comment sync brings it when the issue is opened next.
        log::info!("Issue {issue_id} wasn't fetched again after a comment: {e}");
        db.write(move |tx, changes| {
            remove_pending(tx, &pending_id)?;
            changes.issue_changed(&issue_id);
            Ok(())
        })
        .await?;
    }
    Ok(())
}

/// The comment as the activity shows it until Jira's arrives.
struct Shown {
    issue_id: String,
    pending_id: String,
    html: String,
    adf: String,
    text: String,
    at: String,
}

fn show(conn: &Connection, me: &Me, shown: &Shown) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.comment (id, issue_id, author_id, body_html, body_text, created_at, updated_at, body_adf)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7)",
        params![
            shown.pending_id,
            shown.issue_id,
            me.account_id,
            shown.html,
            shown.text,
            shown.at,
            shown.adf
        ],
    )?;
    // Counted already, so the fetch that brings Jira's comment records no change of mine.
    conn.execute(
        "UPDATE cache.issue SET comment_count = comment_count + 1 WHERE id = ?1",
        [&shown.issue_id],
    )?;
    Ok(())
}

/// Jira refused: the comment and its count go.
fn withdraw(conn: &Connection, issue_id: &str, pending_id: &str) -> AppResult<()> {
    remove_pending(conn, pending_id)?;
    conn.execute(
        "UPDATE cache.issue SET comment_count = MAX(comment_count - 1, 0) WHERE id = ?1",
        [issue_id],
    )?;
    Ok(())
}

fn remove_pending(conn: &Connection, pending_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM cache.comment WHERE id = ?1", [pending_id])?;
    Ok(())
}
