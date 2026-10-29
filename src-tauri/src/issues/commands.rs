//! The Tauri commands for one issue.

use chrono::Utc;
use tauri::State;

use super::{ActivityItem, IssueDetail, Related, WebLink};
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::state::AppState;

fn viewer(state: &AppState) -> Viewer {
    Viewer::of(&state.settings.get().connection, Utc::now())
}

#[tauri::command]
pub async fn issue_get(state: State<'_, AppState>, id: String) -> AppResult<IssueDetail> {
    super::issue_get(&state.db, viewer(&state), id).await
}

/// Comments and recorded field changes, oldest first.
#[tauri::command]
pub async fn issue_activity(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<ActivityItem>> {
    super::issue_activity(&state.db, viewer(&state), id).await
}

/// Removes a gone issue from the cache.
#[tauri::command]
pub async fn issue_forget(state: State<'_, AppState>, id: String) -> AppResult<()> {
    super::issue_forget(&state.db, id).await
}

/// Children (an epic's issues, a task's subtasks) and links.
#[tauri::command]
pub async fn issue_related(state: State<'_, AppState>, id: String) -> AppResult<Related> {
    super::issue_related(&state.db, viewer(&state), id).await
}

/// Fetches the issue's comments when the cached ones may be behind; true when it fetched.
#[tauri::command]
pub async fn issue_sync_comments(state: State<'_, AppState>, id: String) -> AppResult<bool> {
    let connection = state.settings.get().connection;
    let source = crate::sync::source_for(&state.jira, &connection)?;
    super::issue_sync_comments(&state.db, source.as_ref(), id).await
}

/// The cached web links (pull requests, documents).
#[tauri::command]
pub async fn issue_web_links(state: State<'_, AppState>, id: String) -> AppResult<Vec<WebLink>> {
    super::issue_web_links(&state.db, id).await
}

/// Fetches the issue's web links; true when they changed.
#[tauri::command]
pub async fn issue_sync_web_links(state: State<'_, AppState>, id: String) -> AppResult<bool> {
    let connection = state.settings.get().connection;
    let source = crate::sync::source_for(&state.jira, &connection)?;
    super::issue_sync_web_links(&state.db, source.as_ref(), id).await
}
