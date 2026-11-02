//! Changes to several picked issues at once: what every one of them allows, then one write after another through
//! `apply`, so each is checked, recorded and settled like a single change.

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Change, WriteOutcome, apply, issue_key, issue_project, me_of, transitions};
use crate::db::Db;
use crate::error::AppResult;
use crate::status_style::Style;
use crate::store::settings::Settings;
use crate::sync::source::Source;
use crate::views::rows::{Person, Priority, priority};

/// A change to one of the picked issues.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct IssueChange {
    pub issue_id: String,
    pub change: Change,
}

/// A status every picked issue can reach (or is in), with the moves that get each one there.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct BulkStatus {
    pub status_name: String,
    pub style: Style,
    /// One per issue not in the status yet.
    pub changes: Vec<IssueChange>,
}

/// What a bulk change did, for one toast.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct BulkOutcome {
    /// "Moved to Done: 5 issues", or with refusals "Moved to Done: 4 of 5 issues. NW-1043: …".
    pub done: String,
    #[ts(type = "number")]
    pub changed: usize,
    #[ts(type = "number")]
    pub refused: usize,
    /// Sets each changed issue back to what it had.
    pub reverse: Option<Vec<IssueChange>>,
}

/// The statuses every issue can be moved to, or is in already, matched by name (workflows differ between
/// projects), in the order Jira lists the first issue's moves. A move that needs Jira's own screen isn't offered: it
/// can't be done for several issues. A status all the issues are in already isn't offered either.
pub async fn bulk_statuses(
    db: &Db,
    source: &dyn Source,
    ids: Vec<String>,
) -> AppResult<Vec<BulkStatus>> {
    let mut issues = Vec::new();
    for id in ids {
        let moves: Vec<_> = transitions(db, source, id.clone())
            .await?
            .into_iter()
            .filter(|m| !m.opens_jira)
            .collect();
        let lookup = id.clone();
        let current = db.read(move |conn| status_name(conn, &lookup)).await?;
        issues.push((id, current, moves));
    }
    let mut names: Vec<(String, Style)> = Vec::new();
    for (_, _, moves) in &issues {
        for m in moves {
            if !names.iter().any(|(name, _)| *name == m.status_name) {
                names.push((m.status_name.clone(), m.style));
            }
        }
    }
    Ok(names
        .into_iter()
        .filter_map(|(name, style)| {
            let mut changes = Vec::new();
            for (id, current, moves) in &issues {
                if current.as_deref() == Some(name.as_str()) {
                    continue;
                }
                let found = moves.iter().find(|m| m.status_name == name)?;
                changes.push(IssueChange {
                    issue_id: id.clone(),
                    change: Change::Transition {
                        transition_id: found.id.clone(),
                    },
                });
            }
            (!changes.is_empty()).then_some(BulkStatus {
                status_name: name,
                style,
                changes,
            })
        })
        .collect())
}

/// The priorities every issue's edit screen offers, in the first issue's order.
pub async fn bulk_priorities(source: &dyn Source, ids: Vec<String>) -> AppResult<Vec<Priority>> {
    let mut offered: Option<Vec<String>> = None;
    for id in ids {
        let names = source.priorities(&id).await?;
        offered = Some(match offered {
            None => names,
            Some(kept) => kept
                .into_iter()
                .filter(|name| names.contains(name))
                .collect(),
        });
    }
    Ok(offered
        .unwrap_or_default()
        .iter()
        .map(|name| priority(name))
        .collect())
}

/// Who may be assigned all the issues: me, and whoever is on the team of every one of their Spaces.
pub async fn bulk_assignees(
    db: &Db,
    settings: &Settings,
    ids: Vec<String>,
) -> AppResult<Vec<Person>> {
    let me = me_of(settings).ok();
    db.read(move |conn| {
        let mut projects: Vec<String> = Vec::new();
        for id in &ids {
            let project = issue_project(conn, id)?;
            if !projects.contains(&project) {
                projects.push(project);
            }
        }
        let mut people: Option<Vec<Person>> = None;
        for project in projects {
            let team = crate::team::active(conn, &[project], me.as_ref())?;
            people = Some(match people {
                None => team,
                Some(kept) => kept
                    .into_iter()
                    .filter(|p| team.iter().any(|t| t.account_id == p.account_id))
                    .collect(),
            });
        }
        Ok(people.unwrap_or_default())
    })
    .await
}

/// Sends the changes one after another; a refusal doesn't stop the rest. `what` names the change for the toast
/// ("Moved to Done"); `total` counts the picked issues, also those that needed no change.
pub async fn apply_many(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    changes: Vec<IssueChange>,
    what: String,
    total: usize,
) -> AppResult<BulkOutcome> {
    let mut reverse = Vec::new();
    let mut refusals: Vec<String> = Vec::new();
    for IssueChange { issue_id, change } in changes {
        match apply(db, source, settings, issue_id.clone(), change).await {
            Ok(WriteOutcome { reverse: back, .. }) => {
                if let Some(back) = back {
                    reverse.push(IssueChange {
                        issue_id,
                        change: back.change,
                    });
                }
            }
            Err(e) => {
                let id = issue_id.clone();
                let key = db
                    .read(move |conn| issue_key(conn, &id))
                    .await
                    .unwrap_or(issue_id);
                refusals.push(format!("{key}: {e}"));
            }
        }
    }
    let refused = refusals.len();
    let changed = total.saturating_sub(refused);
    let issues = |n: usize| {
        if n == 1 {
            "1 issue".to_string()
        } else {
            format!("{n} issues")
        }
    };
    let done = match refusals.first() {
        None => format!("{what}: {}", issues(total)),
        Some(first) => format!("{what}: {changed} of {}. {first}", issues(total)),
    };
    Ok(BulkOutcome {
        done,
        changed,
        refused,
        reverse: (!reverse.is_empty()).then_some(reverse),
    })
}

fn status_name(conn: &rusqlite::Connection, id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT s.name FROM cache.issue i JOIN cache.status s ON s.id = i.status_id WHERE i.id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?)
}
