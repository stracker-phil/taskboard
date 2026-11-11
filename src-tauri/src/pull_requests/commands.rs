//! Tauri commands of the pull request statuses.

use tauri::State;

use super::IssuePullRequests;
use crate::error::AppResult;
use crate::state::AppState;

/// The issue's pull requests with the statuses already known; runs nothing.
#[tauri::command]
pub async fn issue_pull_requests(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<IssuePullRequests> {
    state
        .pull_requests
        .of_issue(&state.db, id, chrono::Utc::now(), false)
        .await
}

/// Fetches the statuses that are due, then answers like `issue_pull_requests`.
#[tauri::command]
pub async fn issue_sync_pull_requests(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<IssuePullRequests> {
    state
        .pull_requests
        .of_issue(&state.db, id, chrono::Utc::now(), true)
        .await
}
