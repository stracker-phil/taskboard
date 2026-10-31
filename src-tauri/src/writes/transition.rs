//! Moving an issue along its workflow. Jira decides which moves exist, so they are asked for each time; a move
//! that needs Jira's own screen (more fields) opens the issue in Jira instead.

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use super::{Begun, Change, Field, Reverse, WriteOutcome, issue_key, pending, send};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::dto::{Status, Transition};
use crate::status_style::{Style, Styles};
use crate::sync::map::format_utc;
use crate::sync::source::Source;

/// One move the status menu offers, named by the status it leads to.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct TransitionOption {
    pub id: String,
    pub status_id: String,
    pub status_name: String,
    pub style: Style,
    /// Needs Jira's screen: the page opens the issue in Jira.
    pub opens_jira: bool,
}

/// The moves from the issue's status, in Jira's order.
pub async fn transitions(
    db: &Db,
    source: &dyn Source,
    issue_id: String,
) -> AppResult<Vec<TransitionOption>> {
    let moves = source.transitions(&issue_id).await?;
    db.read(move |conn| {
        let project_id = project_of(conn, &issue_id)?;
        let styles = Styles::load(conn)?;
        Ok(moves
            .iter()
            .map(|t| TransitionOption {
                id: t.id.clone(),
                status_id: t.to.id.clone(),
                status_name: t.to.name.clone(),
                style: styles.of(&project_id, &t.to.id, &t.to.status_category.key, &t.to.name),
                opens_jira: t.has_screen,
            })
            .collect())
    })
    .await
}

pub(super) async fn transition(
    db: &Db,
    source: &dyn Source,
    issue_id: String,
    transition_id: String,
) -> AppResult<WriteOutcome> {
    let chosen = chosen_move(source, &issue_id, &transition_id).await?;
    let id = issue_id.clone();
    let target = chosen.clone();
    let now = format_utc(Utc::now());
    let begun = db
        .write(move |tx, changes| {
            remember_status(tx, &target.to)?;
            let begun = begin(tx, &id, &target, &now)?;
            if matches!(begun, Begun::Sent(_)) {
                changes.issue_changed(&id);
            }
            Ok(begun)
        })
        .await?;
    let moved = match begun {
        Begun::AlreadyThere(done) => {
            return Ok(WriteOutcome {
                done,
                reverse: None,
            });
        }
        Begun::Sent(moved) => moved,
    };
    let write = source.transition(&issue_id, &transition_id);
    send(db, source, &issue_id, Field::Status, write).await?;
    let back = reverse_of(source, &issue_id, &moved).await;
    Ok(WriteOutcome {
        done: format!("Moved {} to {}", moved.key, chosen.to.name),
        reverse: back,
    })
}

/// The move as Jira offers it now; one it no longer lists, or one that needs its screen, is refused.
async fn chosen_move(
    source: &dyn Source,
    issue_id: &str,
    transition_id: &str,
) -> AppResult<Transition> {
    let chosen = source
        .transitions(issue_id)
        .await?
        .into_iter()
        .find(|t| t.id == transition_id)
        .ok_or_else(|| {
            AppError::InvalidInput(
                "Jira no longer offers this move; the issue may have changed".into(),
            )
        })?;
    if chosen.has_screen {
        return Err(AppError::InvalidInput(format!(
            "Moving to {} needs Jira's own screen; open the issue in Jira",
            chosen.to.name
        )));
    }
    Ok(chosen)
}

struct Moved {
    key: String,
    before_id: String,
    before_name: String,
}

/// Shows the target status and records the pending write, unless the issue already has it.
fn begin(conn: &Connection, id: &str, target: &Transition, now: &str) -> AppResult<Begun<Moved>> {
    let key = issue_key(conn, id)?;
    let to = &target.to;
    let before = current_status(conn, id)?;
    if before == to.id {
        return Ok(Begun::AlreadyThere(format!("{key} is already {}", to.name)));
    }
    pending::begin(conn, id, Field::Status, Some(&to.id), now)?;
    Ok(Begun::Sent(Moved {
        key,
        before_name: status_name(conn, &before)?,
        before_id: before,
    }))
}

/// The row shows the target status's name and category from `cache.status`, which may not know it yet.
fn remember_status(conn: &Connection, status: &Status) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.status (id, name, category) VALUES (?1, ?2, ?3)
         ON CONFLICT (id) DO UPDATE SET name = excluded.name, category = excluded.category",
        params![status.id, status.name, status.status_category.key],
    )?;
    Ok(())
}

/// The move back to the old status, when the workflow offers one without a screen.
async fn reverse_of(source: &dyn Source, issue_id: &str, moved: &Moved) -> Option<Reverse> {
    let moves = match source.transitions(issue_id).await {
        Ok(moves) => moves,
        Err(e) => {
            log::info!("The moves back for issue {issue_id} weren't fetched: {e}");
            return None;
        }
    };
    moves
        .into_iter()
        .find(|t| t.to.id == moved.before_id && !t.has_screen)
        .map(|t| Reverse {
            label: format!("Change back to {}", moved.before_name),
            change: Change::Transition {
                transition_id: t.id,
            },
        })
}

fn current_status(conn: &Connection, id: &str) -> AppResult<String> {
    Ok(conn.query_row(
        "SELECT status_id FROM cache.issue WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?)
}

fn status_name(conn: &Connection, status_id: &str) -> AppResult<String> {
    Ok(conn
        .query_row(
            "SELECT name FROM cache.status WHERE id = ?1",
            [status_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| status_id.to_string()))
}

fn project_of(conn: &Connection, id: &str) -> AppResult<String> {
    conn.query_row(
        "SELECT project_id FROM cache.issue WHERE id = ?1",
        [id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound("The issue isn't in the cache".into()))
}
