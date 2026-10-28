//! The Tauri commands of the Statuses settings page.

use tauri::State;

use super::{StatusStyleRow, Style};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn status_styles_list(state: State<'_, AppState>) -> AppResult<Vec<StatusStyleRow>> {
    super::status_styles_list(&state.db).await
}

/// `style` null: Automatic.
#[tauri::command]
pub async fn status_style_set(
    state: State<'_, AppState>,
    status_id: String,
    style: Option<Style>,
) -> AppResult<Vec<StatusStyleRow>> {
    super::status_style_set(&state.db, status_id, style).await
}
