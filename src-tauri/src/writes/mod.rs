//! Changes Taskboard makes in the tracker; the rules are in `docs/writes.md`.

pub mod commands;
mod pending;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use pending::{forget_all, overlay};

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::store::settings::Settings;
use crate::sync::map::format_utc;
use crate::sync::refetch_issue;
use crate::sync::source::Source;

/// A change to one issue, as the page asks for it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Change {
    /// Assigns the issue to the account, or unassigns it (`None`).
    Assign { account_id: Option<String> },
}

/// What a write did, for the page's toast.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct WriteOutcome {
    /// "Assigned NW-1042 to you".
    pub done: String,
    /// The opposite change, when there is one.
    pub reverse: Option<Reverse>,
}

/// A second change that sets the field back: a new write in the tracker's history, not an undo.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Reverse {
    /// "Change back to Jamie Doe", "Change back to Unassigned".
    pub label: String,
    pub change: Change,
}

/// A field a write may hold back from the sync (`cache.pending_write.field`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Assignee,
}

impl Field {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assignee => "assignee",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "assignee" => Some(Self::Assignee),
            _ => None,
        }
    }
}

/// Who the connected account is, from the last Test Connection.
#[derive(Clone, Debug)]
struct Me {
    account_id: String,
    name: String,
}

impl Me {
    fn of(settings: &Settings) -> AppResult<Self> {
        let connection = &settings.connection;
        let account_id = connection.account_id.clone().ok_or_else(|| {
            AppError::NotConnected(
                "Test the connection in Settings first, so Taskboard knows who you are".into(),
            )
        })?;
        Ok(Self {
            name: connection.display_name.clone().unwrap_or_default(),
            account_id,
        })
    }
}

/// Sends the change while changes to Jira are allowed.
pub async fn apply(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
    change: Change,
) -> AppResult<WriteOutcome> {
    ensure_allowed(settings)?;
    let me = Me::of(settings)?;
    let Change::Assign { account_id } = change;
    assign(db, source, me, issue_id, account_id).await
}

fn ensure_allowed(settings: &Settings) -> AppResult<()> {
    if settings.behaviour.allow_jira_changes {
        Ok(())
    } else {
        Err(AppError::InvalidInput(
            "Changes to Jira are off. Turn them on in Settings → Connection.".into(),
        ))
    }
}

/// Assigns the issue to me, or unassigns it when it already is mine.
pub async fn toggle_mine(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
) -> AppResult<WriteOutcome> {
    ensure_allowed(settings)?;
    let me = Me::of(settings)?;
    let id = issue_id.clone();
    let current = db.read(move |conn| assignee_of(conn, &id)).await?;
    let target = (current.as_deref() != Some(me.account_id.as_str())).then_some(me.account_id);
    apply(
        db,
        source,
        settings,
        issue_id,
        Change::Assign { account_id: target },
    )
    .await
}

async fn assign(
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
            let begun = begin_assign(tx, &me, &id, target.as_deref(), &now)?;
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
    send(db, source, &issue_id, Field::Assignee, || {
        source.assign(&issue_id, account_id.as_deref())
    })
    .await?;
    Ok(assignment.outcome())
}

/// Shows the new assignee and records the pending write, unless the issue already has it.
fn begin_assign(
    conn: &Connection,
    me: &Me,
    id: &str,
    target: Option<&str>,
    now: &str,
) -> AppResult<Begun> {
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

/// Sends the write once; a failure puts the old value back, a success fetches the issue again and settles.
async fn send<F, Fut>(
    db: &Db,
    source: &dyn Source,
    issue_id: &str,
    field: Field,
    write: F,
) -> AppResult<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = AppResult<()>>,
{
    let id = issue_id.to_string();
    if let Err(e) = write().await {
        log::info!(
            "Jira refused a change of {} on issue {id}: {e}",
            field.as_str()
        );
        db.write(move |tx, changes| {
            pending::roll_back(tx, &id, field)?;
            changes.issue_changed(&id);
            Ok(())
        })
        .await?;
        return Err(e);
    }
    let settle_id = id.clone();
    let refetched = refetch_issue(db, source, &id, move |tx| {
        pending::settle(tx, &settle_id, field)
    })
    .await;
    if let Err(e) = refetched {
        // The change is in Jira; the next sync brings the rest.
        log::info!("Issue {id} wasn't fetched again after a change: {e}");
        db.write(move |tx, _| pending::settle(tx, &id, field))
            .await?;
    }
    Ok(())
}

enum Begun {
    /// Nothing to send; what the toast says instead.
    AlreadyThere(String),
    Sent(Assignment),
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

fn issue_key(conn: &Connection, id: &str) -> AppResult<String> {
    conn.query_row(
        "SELECT key FROM cache.issue WHERE id = ?1 AND gone = 0",
        [id],
        |row| row.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound("The issue isn't in the cache".into()))
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

/// Rows name the assignee from `cache.user`, which may not know the connected account yet.
fn remember_me(conn: &Connection, me: &Me) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.user (account_id, name) VALUES (?1, ?2) ON CONFLICT (account_id) DO NOTHING",
        params![me.account_id, me.name],
    )?;
    Ok(())
}
