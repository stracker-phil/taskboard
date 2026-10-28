//! The Tauri command for the command palette.

use tauri::State;

use super::PaletteResults;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn palette_query(state: State<'_, AppState>, text: String) -> AppResult<PaletteResults> {
    super::palette_query(&state.db, text).await
}
