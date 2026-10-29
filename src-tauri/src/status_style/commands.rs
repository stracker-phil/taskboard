//! The Tauri commands of a Space's Statuses settings.

use tauri::State;

use super::{StatusStyleRow, Style};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn status_styles_list(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<StatusStyleRow>> {
    super::status_styles_list(&state.db, project_id).await
}

/// `style` null: Automatic.
#[tauri::command]
pub async fn status_style_set(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    style: Option<Style>,
) -> AppResult<Vec<StatusStyleRow>> {
    super::status_style_set(&state.db, project_id, status_id, style).await
}
