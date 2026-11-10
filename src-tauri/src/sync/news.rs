//! What a stored issue means for Unread: the changes the owner hasn't made themselves, or, on a project's first
//! sync, what happened since the owner last looked.

use rusqlite::{Connection, params};

use super::diff::{ChangeRow, Snapshot, diff};
use super::map::{IssueRow, Mapped};
use crate::changes::{catch_up, seen_at, seen_until_updated};
use crate::db::Changes;
use crate::error::AppResult;
use crate::jira::dto::Issue;

/// How a page's issues count for Unread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// Changes against the cached issue are recorded.
    Later,
    /// A project's first sync with no issue read yet (a new Space): every issue counts as seen.
    FirstOfNewProject,
    /// A project's first sync after the owner read some of its issues (a fresh cache): what was updated or
    /// created since counts as unseen.
    FirstOfKnownProject,
}

impl Pass {
    /// A new Space starts all read.
    pub fn records_news(self) -> bool {
        self != Self::FirstOfNewProject
    }
}

/// The pass of a project's page: `carried` from its first page, else known when the owner read one of its issues.
pub fn page_pass(conn: &Connection, carried: Option<Pass>, issues: &[Issue]) -> AppResult<Pass> {
    if let Some(pass) = carried {
        return Ok(pass);
    }
    let ids: Vec<&str> = issues.iter().map(|issue| issue.id.as_str()).collect();
    let known: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM json_each(?1) j JOIN issue_seen s ON s.issue_id = j.value)",
        [serde_json::to_string(&ids)?],
        |r| r.get(0),
    )?;
    Ok(if known {
        Pass::FirstOfKnownProject
    } else {
        Pass::FirstOfNewProject
    })
}

/// An issue on its way into the cache.
pub struct Arrival<'a> {
    pub row: &'a IssueRow,
    /// The cached version's tracked fields; `None` for an issue new to the cache.
    pub previous: Option<&'a Snapshot>,
    /// The authors of its fetched comments the cache didn't hold.
    pub new_comments: &'a [Option<String>],
    pub pass: Pass,
    /// The connected account.
    pub me: Option<&'a str>,
    pub now: &'a str,
}

/// Records the arrival's news as `issue_change` rows, or marks it seen.
pub fn record_news(conn: &Connection, changes: &mut Changes, arrival: Arrival) -> AppResult<()> {
    let Arrival {
        row,
        previous,
        new_comments,
        pass,
        me,
        now,
    } = arrival;
    match (pass, previous) {
        (Pass::FirstOfNewProject, _) => {
            conn.execute(
                "INSERT INTO issue_seen (issue_id, seen_at) VALUES (?1, ?2)
                 ON CONFLICT (issue_id) DO NOTHING",
                params![row.id, now],
            )?;
            Ok(())
        }
        (Pass::FirstOfKnownProject, None) => record_since_seen(conn, changes, row, me),
        (_, previous) => {
            let theirs = not_mine(diff(previous, row), row, new_comments, me);
            if theirs.dropped {
                catch_up(conn, changes, &row.id)?;
            }
            record_changes(conn, &row.id, &theirs.changes)
        }
    }
}

/// The authors of the fetched comments the cache doesn't hold yet.
pub fn new_comment_authors(conn: &Connection, mapped: &Mapped) -> AppResult<Vec<Option<String>>> {
    let mut query = conn.prepare_cached("SELECT 1 FROM cache.comment WHERE id = ?1")?;
    let mut authors = Vec::new();
    for comment in &mapped.comments {
        if !query.exists([&comment.id])? {
            authors.push(comment.author_id.clone());
        }
    }
    Ok(authors)
}

struct Theirs {
    changes: Vec<ChangeRow>,
    /// Some of the changes were the owner's own.
    dropped: bool,
}

/// Leaves out an issue the owner created and comments that are all the owner's.
fn not_mine(
    found: Vec<ChangeRow>,
    row: &IssueRow,
    new_comments: &[Option<String>],
    me: Option<&str>,
) -> Theirs {
    let Some(me) = me else {
        return Theirs {
            changes: found,
            dropped: false,
        };
    };
    let all_comments_mine = |change: &ChangeRow| {
        let added = change.added_comments();
        added > 0
            && new_comments.len() as i64 >= added
            && new_comments.iter().all(|a| a.as_deref() == Some(me))
    };
    let mine = |change: &ChangeRow| match change.field {
        "created" => row.reporter_id.as_deref() == Some(me),
        "comment_added" => all_comments_mine(change),
        _ => false,
    };
    let before = found.len();
    let changes: Vec<ChangeRow> = found.into_iter().filter(|c| !mine(c)).collect();
    Theirs {
        dropped: changes.len() < before,
        changes,
    }
}

/// A fresh cache has no versions to compare with: an issue updated after the owner last saw it is "updated",
/// one never seen is "created", unless the owner created it.
fn record_since_seen(
    conn: &Connection,
    changes: &mut Changes,
    row: &IssueRow,
    me: Option<&str>,
) -> AppResult<()> {
    let change = |field| ChangeRow {
        field,
        old_value: None,
        new_value: (field == "created").then(|| row.key.clone()),
        changed_at: row.updated_at.clone(),
    };
    match seen_at(conn, &row.id)? {
        Some(seen) if row.updated_at > seen => record_changes(conn, &row.id, &[change("updated")]),
        Some(_) => Ok(()),
        None if me.is_some() && row.reporter_id.as_deref() == me => {
            seen_until_updated(conn, changes, &row.id)
        }
        None => record_changes(conn, &row.id, &[change("created")]),
    }
}

fn record_changes(conn: &Connection, issue_id: &str, found: &[ChangeRow]) -> AppResult<()> {
    for change in found {
        conn.execute(
            "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                issue_id,
                change.field,
                change.old_value,
                change.new_value,
                change.changed_at
            ],
        )?;
    }
    Ok(())
}
