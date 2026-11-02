//! The Tauri commands that stamp an issue's last view and last interaction.

use chrono::Utc;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;

/// The Issue view shows the issue.
#[tauri::command]
pub async fn issue_viewed(state: State<'_, AppState>, id: String) -> AppResult<()> {
    super::issue_viewed(&state.db, id, Utc::now()).await
}

/// The preview stayed on the issue as long as it takes to mark it read.
#[tauri::command]
pub async fn issue_previewed(state: State<'_, AppState>, id: String) -> AppResult<()> {
    super::issue_previewed(&state.db, id, Utc::now()).await
}
