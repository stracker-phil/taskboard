//! The Tauri commands that change Jira.

use tauri::State;

use super::{
    Assignees, Change, DropOutcome, DropTargets, TransitionOption, ViewGroup, WriteOutcome,
};
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

/// Adds a web link; without a title the link is named after its pull request or address.
#[tauri::command]
pub async fn issue_add_web_link(
    state: State<'_, AppState>,
    id: String,
    url: String,
    title: Option<String>,
) -> AppResult<WriteOutcome> {
    let settings = state.settings.get();
    let source = crate::sync::source_for(&state.jira, &settings.connection)?;
    super::add_web_link(&state.db, source.as_ref(), &settings, id, url, title).await
}

/// The groups of a view grouped by `group_by` that may take the dragged issue.
#[tauri::command]
pub async fn issue_drop_targets(
    state: State<'_, AppState>,
    id: String,
    group_by: GroupBy,
    from_key: String,
    groups: Vec<ViewGroup>,
) -> AppResult<DropTargets> {
    let settings = state.settings.get();
    let source = crate::sync::source_for(&state.jira, &settings.connection)?;
    let found = super::drop_targets(&state.db, source.as_ref(), &settings, id, group_by).await?;
    Ok(super::arrange(found, &from_key, &groups))
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

/// Who the issue may be assigned to.
#[tauri::command]
pub async fn issue_assignees(state: State<'_, AppState>, id: String) -> AppResult<Assignees> {
    let me = crate::team::Me::of(&state.settings.get().connection);
    super::assignees(&state.db, id, me).await
}
