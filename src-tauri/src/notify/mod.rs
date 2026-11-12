//! macOS notifications for changes that concern the owner, after a change sync (`docs/notifications.md`).
//! `Notifier` remembers which recorded changes it has looked at; `notices` decides what to say (pure SQL, tested
//! on a temp cache) by the macOS column of Settings → Notifications (`rules`); `show` hands them to macOS.

pub mod commands;
pub mod people;
pub mod rules;
pub mod settings;
pub mod show;

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};

use crate::changes::{Change, Names, facts, sentence};
use crate::db::Db;
use crate::error::AppResult;
use crate::lock::MutexExt;
use crate::store::settings::{Channel, Notifications, Settings};
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
        let notifications = settings.notifications.clone();
        let (newest, notices) = db
            .read(move |conn| {
                let newest = newest_change(conn)?;
                // A reset cache numbers its changes from 1 again.
                let after = if newest < looked_at { 0 } else { looked_at };
                let notices = match (&me, announce) {
                    (Some(me), true) if !notifications.mac.is_empty() => {
                        notices(conn, after, me, &notifications)?
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
    /// The changes that count for macOS, in the order they were recorded.
    changes: Vec<Change>,
}

/// Notices for the changes after `after` that the macOS column counts, summarized past `MAX_SINGLE`.
pub fn notices(
    conn: &Connection,
    after: i64,
    me: &str,
    settings: &Notifications,
) -> AppResult<Vec<Notice>> {
    let issues = changed_issues(conn, after, me, settings)?;
    let statuses = names_of(conn, "SELECT id, name FROM cache.status")?;
    let people = names_of(conn, "SELECT account_id, name FROM cache.user")?;
    let fields = names_of(conn, "SELECT id, name FROM cache.field")?;
    let names = Names {
        me: Some(me),
        statuses: &statuses,
        people: &people,
        fields: &fields,
    };
    let said = issues
        .into_values()
        .filter_map(|issue| notice_for(issue, &names))
        .collect();
    Ok(summarized(said))
}

/// The live issues with changes after `after` that count, by id.
fn changed_issues(
    conn: &Connection,
    after: i64,
    me: &str,
    settings: &Notifications,
) -> AppResult<BTreeMap<String, Issue>> {
    let mut issue =
        conn.prepare_cached("SELECT key, summary FROM cache.issue WHERE id = ?1 AND gone = 0")?;
    let mut issues: BTreeMap<String, Issue> = BTreeMap::new();
    for judged in rules::judged(conn, rules::Which::After(after), Some(me))? {
        if !judged.counts(settings, Channel::Mac, Some(me)) {
            continue;
        }
        let found = match issues.entry(judged.issue_id) {
            Entry::Occupied(known) => known.into_mut(),
            Entry::Vacant(new) => {
                let Some((key, summary)) = issue
                    .query_row([new.key()], |r| Ok((r.get(0)?, r.get(1)?)))
                    .optional()?
                else {
                    continue;
                };
                new.insert(Issue {
                    key,
                    summary,
                    changes: Vec::new(),
                })
            }
        };
        found.changes.push(judged.change);
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
