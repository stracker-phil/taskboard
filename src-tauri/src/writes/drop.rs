//! Dragging an issue onto another group of a view: the drop changes the field the view groups by, when Taskboard
//! can write it and Jira's workflow allows it.

use serde::Serialize;
use ts_rs::TS;

use super::transition::{TransitionOption, transitions};
use super::{Change, WriteOutcome, apply};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::store::settings::Settings;
use crate::sync::source::Source;
use crate::views::GroupBy;
use crate::views::model::to_text;

/// Which groups of a view may take the dragged issue.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DropTargets {
    /// Every group takes it (grouped by assignee).
    pub any: bool,
    /// The group keys that take it, when not `any`.
    pub keys: Vec<String>,
}

/// What a drop did, or the moves to choose from when several statuses of the dropped-on type can follow.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DropOutcome {
    Done { outcome: WriteOutcome },
    Choose { moves: Vec<TransitionOption> },
}

/// The groups the issue may be dropped on; none while changes to Jira are off. Status groups come from the moves
/// Jira offers now, without the ones that need its screen.
pub async fn drop_targets(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
    group_by: GroupBy,
) -> AppResult<DropTargets> {
    if !settings.behaviour.allow_jira_changes {
        return Ok(DropTargets::none());
    }
    Ok(match group_by {
        GroupBy::Assignee => DropTargets {
            any: true,
            keys: Vec::new(),
        },
        GroupBy::Status => DropTargets::of(
            free_moves(db, source, issue_id)
                .await?
                .into_iter()
                .map(|m| m.status_id)
                .collect(),
        ),
        GroupBy::StatusType => {
            let mut types: Vec<String> = free_moves(db, source, issue_id)
                .await?
                .iter()
                .map(status_type_key)
                .collect();
            types.sort();
            types.dedup();
            DropTargets::of(types)
        }
        _ => DropTargets::none(),
    })
}

impl DropTargets {
    fn none() -> Self {
        Self::of(Vec::new())
    }

    fn of(keys: Vec<String>) -> Self {
        Self { any: false, keys }
    }
}

/// The key of the status-type group the move leads into (`views::groups`).
fn status_type_key(m: &TransitionOption) -> String {
    to_text(m.style)
}

/// Status and status-type drops take a move that doesn't need Jira's screen; an empty assignee key unassigns.
pub async fn drop_on(
    db: &Db,
    source: &dyn Source,
    settings: &Settings,
    issue_id: String,
    group_by: GroupBy,
    key: String,
) -> AppResult<DropOutcome> {
    let change = match group_by {
        GroupBy::Assignee => Change::Assign {
            account_id: Some(key).filter(|k| !k.is_empty()),
        },
        GroupBy::Status => {
            let moves = free_moves(db, source, issue_id.clone()).await?;
            let Some(chosen) = moves.into_iter().find(|m| m.status_id == key) else {
                return Err(not_allowed());
            };
            Change::Transition {
                transition_id: chosen.id,
            }
        }
        GroupBy::StatusType => {
            let mut moves: Vec<TransitionOption> = free_moves(db, source, issue_id.clone())
                .await?
                .into_iter()
                .filter(|m| status_type_key(m) == key)
                .collect();
            match moves.len() {
                0 => return Err(not_allowed()),
                1 => Change::Transition {
                    transition_id: moves.remove(0).id,
                },
                _ => return Ok(DropOutcome::Choose { moves }),
            }
        }
        _ => {
            return Err(AppError::InvalidInput(
                "Issues can't be moved between these groups".into(),
            ));
        }
    };
    let outcome = apply(db, source, settings, issue_id, change).await?;
    Ok(DropOutcome::Done { outcome })
}

/// The moves that don't need Jira's own screen.
async fn free_moves(
    db: &Db,
    source: &dyn Source,
    issue_id: String,
) -> AppResult<Vec<TransitionOption>> {
    Ok(transitions(db, source, issue_id)
        .await?
        .into_iter()
        .filter(|m| !m.opens_jira)
        .collect())
}

fn not_allowed() -> AppError {
    AppError::InvalidInput("Jira's workflow doesn't allow this move for the issue".into())
}
