//! The Tauri commands for marking issues read (seen) and unread.

use chrono::Utc;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;
use crate::time::format_utc;

#[tauri::command]
pub async fn issues_mark_seen(state: State<'_, AppState>, ids: Vec<String>) -> AppResult<()> {
    super::mark_seen(&state.db, ids, format_utc(Utc::now())).await
}

/// The M key on a row: read when it is unread, else unread again.
#[tauri::command]
pub async fn issue_toggle_seen(state: State<'_, AppState>, id: String) -> AppResult<()> {
    super::toggle_seen(&state.db, id, format_utc(Utc::now())).await
}

/// Unread again: from the activity item at `from` on, or the whole issue.
#[tauri::command]
pub async fn issue_mark_unread(
    state: State<'_, AppState>,
    id: String,
    from: Option<String>,
) -> AppResult<()> {
    super::mark_unread(&state.db, id, from).await
}
