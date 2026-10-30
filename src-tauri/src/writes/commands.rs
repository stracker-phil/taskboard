//! The Tauri commands that change Jira.

use tauri::State;

use super::{Change, WriteOutcome};
use crate::error::AppResult;
use crate::state::AppState;

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
