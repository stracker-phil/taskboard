//! The Tauri commands that change Jira.

use tauri::State;

use super::{Change, DropOutcome, DropTargets, TransitionOption, WriteOutcome};
use crate::error::AppResult;
use crate::state::AppState;
use crate::views::GroupBy;

#[tauri::command]
pub async fn issue_write(
    state: State<'_, AppState>,
    id: String,
    change: Change,
) -> AppResult<WriteOutcome> {
    let settings = state.settings.get();
    let source = crate::sync::source_for(&state.jira, &settings.connection)?;
    super::apply(&state.db, source.as_ref(), &settings, id, change).await
}

/// Assigns the issue to me, or unassigns it when it is mine.
#[tauri::command]
pub async fn issue_toggle_mine(state: State<'_, AppState>, id: String) -> AppResult<WriteOutcome> {
    let settings = state.settings.get();
    let source = crate::sync::source_for(&state.jira, &settings.connection)?;
    super::toggle_mine(&state.db, source.as_ref(), &settings, id).await
}

/// The moves Jira offers from the issue's status.
#[tauri::command]
pub async fn issue_transitions(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<TransitionOption>> {
    let settings = state.settings.get();
    let source = crate::sync::source_for(&state.jira, &settings.connection)?;
    super::transitions(&state.db, source.as_ref(), id).await
}

/// Adds a comment, typed as plain text.
#[tauri::command]
pub async fn issue_comment(
    state: State<'_, AppState>,
    id: String,
    text: String,
) -> AppResult<WriteOutcome> {
    let settings = state.settings.get();
    let source = crate::sync::source_for(&state.jira, &settings.connection)?;
    super::add_comment(&state.db, source.as_ref(), &settings, id, text).await
}

/// The groups of a view grouped by `group_by` that may take the dragged issue.
#[tauri::command]
pub async fn issue_drop_targets(
    state: State<'_, AppState>,
    id: String,
    group_by: GroupBy,
) -> AppResult<DropTargets> {
    let settings = state.settings.get();
    let source = crate::sync::source_for(&state.jira, &settings.connection)?;
    super::drop_targets(&state.db, source.as_ref(), &settings, id, group_by).await
}

/// Moves the issue into the group `key` of a view grouped by `group_by`.
#[tauri::command]
pub async fn issue_drop(
    state: State<'_, AppState>,
    id: String,
    group_by: GroupBy,
    key: String,
) -> AppResult<DropOutcome> {
    let settings = state.settings.get();
    let source = crate::sync::source_for(&state.jira, &settings.connection)?;
    super::drop_on(&state.db, source.as_ref(), &settings, id, group_by, key).await
}
