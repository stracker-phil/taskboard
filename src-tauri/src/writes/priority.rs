//! Setting an issue's priority to one Jira offers on the issue's edit screen.

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use super::{Begun, Change, Field, Offers, Reverse, WriteOutcome, issue_key, pending, send};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::sync::source::Source;
use crate::time::format_utc;
use crate::views::rows::{Priority, priority};

/// What the priority menu offers: the priorities Jira allows on the issue's edit screen, and the one it has.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Priorities {
    pub options: Vec<Priority>,
    pub current: Option<String>,
}

pub async fn priorities(
    db: &Db,
    source: &dyn Source,
    offers: &Offers,
    issue_id: String,
) -> AppResult<Priorities> {
    let names = offers.priorities(db, source, &issue_id).await?;
    let current = db
        .read(move |conn| current_priority(conn, &issue_id))
        .await?;
    Ok(Priorities {
        options: names.iter().map(|name| priority(name)).collect(),
        current,
    })
}

pub(super) async fn set_priority(
    db: &Db,
    source: &dyn Source,
    offers: &Offers,
    issue_id: String,
    name: String,
) -> AppResult<WriteOutcome> {
    if !offers
        .priorities(db, source, &issue_id)
        .await?
        .contains(&name)
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_serializes_options_with_levels_and_the_current_name() {
        let menu = Priorities {
            options: vec![priority("Highest"), priority("Low")],
            current: Some("Low".into()),
        };

        assert_eq!(
            serde_json::to_value(&menu).unwrap(),
            serde_json::json!({
                "options": [
                    { "name": "Highest", "level": 1 },
                    { "name": "Low", "level": 4 },
                ],
                "current": "Low",
            })
        );
    }

    #[test]
    fn a_menu_without_a_current_priority_serializes_null() {
        let menu = Priorities {
            options: Vec::new(),
            current: None,
        };

        assert_eq!(
            serde_json::to_value(&menu).unwrap(),
            serde_json::json!({ "options": [], "current": null })
        );
    }
}
