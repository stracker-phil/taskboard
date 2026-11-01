//! Setting an issue's priority to one Jira offers on the issue's edit screen.

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};

use super::{Begun, Change, Field, Reverse, WriteOutcome, issue_key, pending, send};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::sync::map::format_utc;
use crate::sync::source::Source;

pub(super) async fn set_priority(
    db: &Db,
    source: &dyn Source,
    issue_id: String,
    name: String,
) -> AppResult<WriteOutcome> {
    if !source.priorities(&issue_id).await?.contains(&name) {
        return Err(AppError::InvalidInput(format!(
            "Jira doesn't offer the priority {name} for this issue"
        )));
    }
    let id = issue_id.clone();
    let target = name.clone();
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
    let write = source.set_priority(&issue_id, &name);
    let sent = format!("Priority: {name}");
    send(db, source, &issue_id, Field::Priority, sent, write).await?;
    Ok(change.outcome(&name))
}

struct PriorityChange {
    key: String,
    before: Option<String>,
}

impl PriorityChange {
    fn outcome(self, name: &str) -> WriteOutcome {
        WriteOutcome {
            done: format!("Set {} to {name} priority", self.key),
            reverse: self.before.map(|before| Reverse {
                label: format!("Change back to {before}"),
                change: Change::Priority { name: before },
            }),
        }
    }
}

fn begin(conn: &Connection, id: &str, name: &str, now: &str) -> AppResult<Begun<PriorityChange>> {
    let key = issue_key(conn, id)?;
    if current_priority(conn, id)?.as_deref() == Some(name) {
        return Ok(Begun::AlreadyThere(format!(
            "{key} already has {name} priority"
        )));
    }
    let before = pending::begin(conn, id, Field::Priority, Some(name), now)?;
    Ok(Begun::Sent(PriorityChange { key, before }))
}

/// The priority the issue row shows; `None` for an issue without one.
fn current_priority(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT priority FROM cache.issue WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .flatten())
}
