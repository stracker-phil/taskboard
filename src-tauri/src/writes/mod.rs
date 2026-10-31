//! Changes Taskboard makes in the tracker; the rules are in `docs/writes.md`.

mod assign;
pub mod commands;
mod comment;
mod pending;
mod transition;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use assign::toggle_mine;
pub use comment::{MAX_COMMENT_CHARS, add_comment};
pub use pending::{forget_all, overlay};
pub use transition::{TransitionOption, transitions};

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::store::settings::Settings;
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
    /// Moves the issue along a workflow transition (`issue_transitions`).
    Transition { transition_id: String },
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
    /// "Change back to Jamie Doe", "Change back to In Progress".
    pub label: String,
    pub change: Change,
}

/// A field a write may hold back from the sync (`cache.pending_write.field`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Assignee,
    Status,
}

impl Field {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assignee => "assignee",
            Self::Status => "status",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "assignee" => Some(Self::Assignee),
            "status" => Some(Self::Status),
            _ => None,
        }
    }
}

/// Who the connected account is, from the last Test Connection.
#[derive(Debug)]
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
    match change {
        Change::Assign { account_id } => {
            assign::assign(db, source, Me::of(settings)?, issue_id, account_id).await
        }
        Change::Transition { transition_id } => {
            transition::transition(db, source, issue_id, transition_id).await
        }
    }
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

/// Sends the write once; a failure puts the old value back, a success fetches the issue again and settles.
async fn send<Fut>(
    db: &Db,
    source: &dyn Source,
    issue_id: &str,
    field: Field,
    write: Fut,
) -> AppResult<()>
where
    Fut: std::future::Future<Output = AppResult<()>>,
{
    let id = issue_id.to_string();
    if let Err(e) = write.await {
        return Err(refused(db, id, field, e).await);
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

/// Puts the old value back and hands on Jira's refusal.
async fn refused(db: &Db, id: String, field: Field, error: AppError) -> AppError {
    log::info!(
        "Jira refused a change of {} on issue {id}: {error}",
        field.as_str()
    );
    let rolled_back = db
        .write(move |tx, changes| {
            pending::roll_back(tx, &id, field)?;
            changes.issue_changed(&id);
            Ok(())
        })
        .await;
    match rolled_back {
        Ok(()) => error,
        Err(e) => e,
    }
}

/// What beginning a write found.
enum Begun<T> {
    /// Nothing to send; what the toast says instead.
    AlreadyThere(String),
    Sent(T),
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

/// Rows name the assignee from `cache.user`, which may not know the connected account yet.
fn remember_me(conn: &Connection, me: &Me) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.user (account_id, name) VALUES (?1, ?2) ON CONFLICT (account_id) DO NOTHING",
        params![me.account_id, me.name],
    )?;
    Ok(())
}
