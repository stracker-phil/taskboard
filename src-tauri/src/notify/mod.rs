//! macOS notifications for changes that concern the owner, after a change sync (`docs/notifications.md`).
//! `Notifier` remembers which recorded changes it has looked at; `notices` decides what to say (pure SQL, tested
//! on a temp cache); `show` hands them to macOS.

pub mod commands;
pub mod show;

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::changes::{Change, Names, facts, sentence};
use crate::db::Db;
use crate::error::AppResult;
use crate::lock::MutexExt;
use crate::store::settings::{NotifyAbout, Settings};
use crate::sync::{Job, SyncState};

/// More issues than this in one sync make a single summary notification.
pub const MAX_SINGLE: usize = 3;

/// One notification.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    pub title: String,
    pub body: String,
    /// The issue to open from it; none for a summary.
    pub key: Option<String>,
}

/// Which sync job ran, from the engine's state events; a job is finished when the engine leaves `running`.
#[derive(Default)]
pub struct FinishedJobs(Mutex<Option<Job>>);

impl FinishedJobs {
    pub fn observe(&self, state: &SyncState) -> Option<Job> {
        let mut running = self.0.lock_unpoisoned();
        match state {
            SyncState::Running { job } => {
                *running = Some(*job);
                None
            }
            _ => running.take(),
        }
    }
}

/// The newest `cache.issue_change` row already looked at; changes after it are news.
#[derive(Clone, Default)]
pub struct Notifier(Arc<AtomicI64>);

impl Notifier {
    /// Starts after everything already recorded: nothing from before the launch is announced.
    pub async fn start(db: &Db) -> AppResult<Self> {
        let newest = db.read(newest_change).await?;
        Ok(Self(Arc::new(AtomicI64::new(newest))))
    }

    /// The notices for the changes recorded since the last call. `announce` false (a full sync, the window has
    /// the focus) passes them by.
    pub async fn take(
        &self,
        db: &Db,
        settings: &Settings,
        announce: bool,
    ) -> AppResult<Vec<Notice>> {
        let looked_at = self.0.load(Ordering::SeqCst);
        let me = settings.connection.account_id.clone();
        let about = settings.behaviour.notify_about;
        let (newest, notices) = db
            .read(move |conn| {
                let newest = newest_change(conn)?;
                // A reset cache numbers its changes from 1 again.
                let after = if newest < looked_at { 0 } else { looked_at };
                let notices = match (&me, announce, about) {
                    (Some(me), true, about) if about != NotifyAbout::Nothing => {
                        notices(conn, after, me, about)?
                    }
                    _ => Vec::new(),
                };
                Ok((newest, notices))
            })
            .await?;
        self.0.store(newest, Ordering::SeqCst);
        Ok(notices)
    }
}

fn newest_change(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(rowid), 0) FROM cache.issue_change",
        [],
        |row| row.get(0),
    )?)
}

struct Issue {
    key: String,
    summary: String,
    changes: Vec<Change>,
    /// Assigned to or reported by the connected account.
    mine: bool,
}

/// Notices for the changes after `after` that concern `me`, summarized past `MAX_SINGLE`.
pub fn notices(
    conn: &Connection,
    after: i64,
    me: &str,
    about: NotifyAbout,
) -> AppResult<Vec<Notice>> {
    let issues = changed_issues(conn, after, me)?;
    let statuses = names_of(conn, "SELECT id, name FROM cache.status")?;
    let people = names_of(conn, "SELECT account_id, name FROM cache.user")?;
    let names = Names {
        me: Some(me),
        statuses: &statuses,
        people: &people,
    };
    let concerning = issues
        .into_values()
        .filter(|issue| concerns(issue, me, about))
        .filter_map(|issue| notice_for(issue, &names))
        .collect();
    Ok(summarized(concerning))
}

/// The issues with changes after `after`, each with them in the order they were recorded.
fn changed_issues(conn: &Connection, after: i64, me: &str) -> AppResult<BTreeMap<String, Issue>> {
    let mut query = conn.prepare(
        "SELECT c.issue_id, i.key, i.summary, c.field, c.old_value, c.new_value, c.changed_at,
           COALESCE(i.assignee_id = ?2 OR i.reporter_id = ?2, 0)
         FROM cache.issue_change c JOIN cache.issue i ON i.id = c.issue_id
         WHERE c.rowid > ?1 AND i.gone = 0 AND c.field NOT IN ('created', 'gone')
         ORDER BY c.rowid",
    )?;
    let mut rows = query.query(rusqlite::params![after, me])?;
    let mut issues: BTreeMap<String, Issue> = BTreeMap::new();
    while let Some(r) = rows.next()? {
        let issue = issues.entry(r.get(0)?).or_insert(Issue {
            key: r.get(1)?,
            summary: r.get(2)?,
            changes: Vec::new(),
            mine: r.get(7)?,
        });
        issue.changes.push(Change {
            field: r.get(3)?,
            old_value: r.get(4)?,
            new_value: r.get(5)?,
            at: r.get(6)?,
        });
    }
    Ok(issues)
}

fn notice_for(issue: Issue, names: &Names) -> Option<Notice> {
    let said = sentence(&facts(&issue.changes), names)?;
    Some(Notice {
        title: format!("{} {}", issue.key, issue.summary),
        body: capitalized(&said),
        key: Some(issue.key),
    })
}

/// A mention of me or an assignment to me always concerns me; with `MyIssues` any change of my issue does.
fn concerns(issue: &Issue, me: &str, about: NotifyAbout) -> bool {
    let for_me = issue.changes.iter().any(|c| {
        matches!(c.field.as_str(), "mention" | "assignee") && c.new_value.as_deref() == Some(me)
    });
    for_me || (about == NotifyAbout::MyIssues && issue.mine)
}

fn summarized(notices: Vec<Notice>) -> Vec<Notice> {
    if notices.len() <= MAX_SINGLE {
        return notices;
    }
    let keys: Vec<&str> = notices.iter().filter_map(|n| n.key.as_deref()).collect();
    vec![Notice {
        title: format!("{} issues changed", notices.len()),
        body: keys.join(", "),
        key: None,
    }]
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Ids to names, from a two-column query.
fn names_of(conn: &Connection, sql: &str) -> AppResult<HashMap<String, String>> {
    let mut query = conn.prepare(sql)?;
    let names = query.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(names.collect::<Result<_, _>>()?)
}
