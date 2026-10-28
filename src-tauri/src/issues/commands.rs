//! The Tauri commands for one issue.

use chrono::Utc;
use tauri::State;

use super::IssueDetail;
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::state::AppState;

#[tauri::command]
pub async fn issue_get(state: State<'_, AppState>, id: String) -> AppResult<IssueDetail> {
    let viewer = Viewer::of(&state.settings.get().connection, Utc::now());
    super::issue_get(&state.db, viewer, id).await
}
