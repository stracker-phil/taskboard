//! Assigning an issue to someone, or to no one.

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use super::{
    Begun, Change, Field, Me, Reverse, WriteOutcome, apply, ensure_allowed, issue_key,
    issue_project, pending, remember_me, send,
};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::capability::Capability;
use crate::store::settings::Settings;
use crate::sync::map::format_utc;
use crate::sync::source::Source;
use crate::views::rows::Person;

/// Assigns the issue to me, or unassigns it when it already is mine.
pub async fn toggle_mine(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
) -> AppResult<WriteOutcome> {
    // Says "changes are off" before asking who I am.
    ensure_allowed(settings, Capability::Assign)?;
    let me = Me::of(settings)?;
    let id = issue_id.clone();
    let current = db.read(move |conn| assignee_of(conn, &id)).await?;
    let already_mine = current.as_deref() == Some(me.account_id.as_str());
    let target = (!already_mine).then_some(me.account_id);
    let change = Change::Assign { account_id: target };
    apply(db, source, settings, issue_id, change).await
}

pub(super) async fn assign(
    db: &Db,
    source: &dyn Source,
    me: Me,
    issue_id: String,
    account_id: Option<String>,
) -> AppResult<WriteOutcome> {
    let id = issue_id.clone();
    let target = account_id.clone();
    let now = format_utc(Utc::now());
    let begun = db
        .write(move |tx, changes| {
            let begun = begin(tx, &me, &id, target.as_deref(), &now)?;
            if matches!(begun, Begun::Sent(_)) {
                changes.issue_changed(&id);
            }
            Ok(begun)
        })
        .await?;
    let assignment = match begun {
        Begun::AlreadyThere(done) => {
            return Ok(WriteOutcome {
                done,
                reverse: None,
            });
        }
        Begun::Sent(assignment) => assignment,
    };
    let write = source.assign(&issue_id, account_id.as_deref());
    send(db, source, &issue_id, Field::Assignee, write).await?;
    Ok(assignment.outcome())
}

/// Shows the new assignee and records the pending write, unless the issue already has it.
fn begin(
    conn: &Connection,
    me: &Me,
    id: &str,
    target: Option<&str>,
    now: &str,
) -> AppResult<Begun<Assignment>> {
    let key = issue_key(conn, id)?;
    if assignee_of(conn, id)?.as_deref() == target {
        let done = match person(conn, me, target)? {
            Some(name) => format!("{key} is already assigned to {name}"),
            None => format!("{key} is already unassigned"),
        };
        return Ok(Begun::AlreadyThere(done));
    }
    remember_me(conn, me)?;
    let before = pending::begin(conn, id, Field::Assignee, target, now)?;
    Ok(Begun::Sent(Assignment {
        key,
        before: person(conn, me, before.as_deref())?,
        after: person(conn, me, target)?,
        before_id: before,
    }))
}

struct Assignment {
    key: String,
    /// "you", a name, or `None` for unassigned.
    before: Option<String>,
    after: Option<String>,
    before_id: Option<String>,
}

impl Assignment {
    fn outcome(self) -> WriteOutcome {
        let done = match &self.after {
            Some(name) => format!("Assigned {} to {name}", self.key),
            None => format!("Unassigned {}", self.key),
        };
        let back_to = self.before.as_deref().unwrap_or("Unassigned");
        WriteOutcome {
            done,
            reverse: Some(Reverse {
                label: format!("Change back to {back_to}"),
                change: Change::Assign {
                    account_id: self.before_id,
                },
            }),
        }
    }
}

fn assignee_of(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    conn.query_row(
        "SELECT assignee_id FROM cache.issue WHERE id = ?1",
        [id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound("The issue isn't in the cache".into()))
}

/// "you" for the connected account, else the cached name (or the id when the cache has no name).
fn person(conn: &Connection, me: &Me, account_id: Option<&str>) -> AppResult<Option<String>> {
    let Some(account_id) = account_id else {
        return Ok(None);
    };
    if account_id == me.account_id {
        return Ok(Some("you".into()));
    }
    let name: Option<String> = conn
        .query_row(
            "SELECT name FROM cache.user WHERE account_id = ?1",
            [account_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(Some(name.unwrap_or_else(|| account_id.to_string())))
}

/// Who an issue may be assigned to, for the assignee menu.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Assignees {
    /// The issue's Space's active team, me first.
    pub people: Vec<Person>,
    /// Who has the issue now.
    pub current: Option<String>,
}

pub async fn assignees(db: &Db, issue_id: String, me: Option<String>) -> AppResult<Assignees> {
    db.read(move |conn| {
        let project = issue_project(conn, &issue_id)?;
        Ok(Assignees {
            people: crate::team::active(conn, &[project], me.as_deref())?,
            current: assignee_of(conn, &issue_id)?,
        })
    })
    .await
}
