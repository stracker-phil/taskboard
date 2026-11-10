//! Jira's moves and priorities for issues, kept for a while so menus, drags and the check before a write don't ask
//! again.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension};
use tokio::time::Instant;

use crate::db::Db;
use crate::error::AppResult;
use crate::jira::dto::Transition;
use crate::lock::MutexExt;
use crate::sync::source::Source;

const MOVES_KEPT: Duration = Duration::from_secs(60);
const PRIORITIES_KEPT: Duration = Duration::from_secs(10 * 60);

/// Issues of one kind share an edit screen.
#[derive(PartialEq, Eq, Hash)]
struct Kind {
    project_id: String,
    issue_type: String,
    status_id: String,
}

/// `clear` when the connection changes.
pub struct Offers {
    /// Per issue and the status it had when asked: a workflow's conditions may look at the issue.
    moves: Shelf<(String, String), Vec<Transition>>,
    priorities: Shelf<Kind, Vec<String>>,
}

impl Default for Offers {
    fn default() -> Self {
        Self {
            moves: Shelf::new(MOVES_KEPT),
            priorities: Shelf::new(PRIORITIES_KEPT),
        }
    }
}

impl Offers {
    pub async fn moves(
        &self,
        db: &Db,
        source: &dyn Source,
        issue_id: &str,
    ) -> AppResult<Vec<Transition>> {
        let id = issue_id.to_string();
        let Some(kind) = db.read(move |conn| kind_of(conn, &id)).await? else {
            return source.transitions(issue_id).await;
        };
        let key = (issue_id.to_string(), kind.status_id);
        if let Some(moves) = self.moves.get(&key) {
            return Ok(moves);
        }
        let moves = source.transitions(issue_id).await?;
        self.moves.put(key, moves.clone());
        Ok(moves)
    }

    /// The priorities the issue's edit screen offers, asked once per kind of issue.
    pub async fn priorities(
        &self,
        db: &Db,
        source: &dyn Source,
        issue_id: &str,
    ) -> AppResult<Vec<String>> {
        let id = issue_id.to_string();
        let Some(kind) = db.read(move |conn| kind_of(conn, &id)).await? else {
            return source.priorities(issue_id).await;
        };
        if let Some(names) = self.priorities.get(&kind) {
            return Ok(names);
        }
        let names = source.priorities(issue_id).await?;
        self.priorities.put(kind, names.clone());
        Ok(names)
    }

    /// Taskboard changed the issue, which a workflow's conditions may look at: its moves are asked afresh.
    pub fn forget_issue(&self, issue_id: &str) {
        self.moves.retain(|(id, _)| id != issue_id);
    }

    pub fn clear(&self) {
        self.moves.clear();
        self.priorities.clear();
    }
}

/// A map whose entries expire `keep` after they were put.
struct Shelf<K, V> {
    keep: Duration,
    items: Mutex<HashMap<K, (Instant, V)>>,
}

impl<K: Eq + Hash, V: Clone> Shelf<K, V> {
    fn new(keep: Duration) -> Self {
        Self {
            keep,
            items: Mutex::default(),
        }
    }

    fn get(&self, key: &K) -> Option<V> {
        let items = self.items.lock_unpoisoned();
        let (put, value) = items.get(key)?;
        (put.elapsed() < self.keep).then(|| value.clone())
    }

    fn put(&self, key: K, value: V) {
        let mut items = self.items.lock_unpoisoned();
        let keep = self.keep;
        items.retain(|_, (put, _)| put.elapsed() < keep);
        items.insert(key, (Instant::now(), value));
    }

    fn retain(&self, mut keep: impl FnMut(&K) -> bool) {
        self.items.lock_unpoisoned().retain(|key, _| keep(key));
    }

    fn clear(&self) {
        self.items.lock_unpoisoned().clear();
    }
}

/// The first of the issues of each kind, in their order; the others share its edit screen. An issue the cache
/// doesn't know counts as a kind of its own.
pub(super) async fn one_per_kind(db: &Db, ids: Vec<String>) -> AppResult<Vec<String>> {
    db.read(move |conn| {
        let mut kinds: Vec<Kind> = Vec::new();
        let mut firsts = Vec::new();
        for id in ids {
            match kind_of(conn, &id)? {
                Some(kind) if kinds.contains(&kind) => {}
                Some(kind) => {
                    kinds.push(kind);
                    firsts.push(id);
                }
                None => firsts.push(id),
            }
        }
        Ok(firsts)
    })
    .await
}

fn kind_of(conn: &Connection, id: &str) -> AppResult<Option<Kind>> {
    Ok(conn
        .query_row(
            "SELECT project_id, issue_type, status_id FROM cache.issue WHERE id = ?1",
            [id],
            |row| {
                Ok(Kind {
                    project_id: row.get(0)?,
                    issue_type: row.get(1)?,
                    status_id: row.get(2)?,
                })
            },
        )
        .optional()?)
}
