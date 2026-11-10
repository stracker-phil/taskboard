//! The Tauri commands of the Statuses settings, app-wide and per Space.

use tauri::State;

use super::{AppStatusRow, StatusStyleRow, Style};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn status_styles_list(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<StatusStyleRow>> {
    super::status_styles_list(&state.db, project_id).await
}

/// `style` null: the app-wide style.
#[tauri::command]
pub async fn status_style_set(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    style: Option<Style>,
) -> AppResult<Vec<StatusStyleRow>> {
    super::status_style_set(&state.db, project_id, status_id, style).await
}

#[tauri::command]
pub async fn status_styles_all(state: State<'_, AppState>) -> AppResult<Vec<AppStatusRow>> {
    super::status_styles_all(&state.db).await
}

/// `style` null: Automatic.
#[tauri::command]
pub async fn status_style_set_app(
    state: State<'_, AppState>,
    status_id: String,
    style: Option<Style>,
) -> AppResult<Vec<AppStatusRow>> {
    super::status_style_set_app(&state.db, status_id, style).await
}
