//! The Tauri command for the command palette.

use tauri::State;

use super::{PaletteResults, PaletteScope, SavedSearch};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn palette_query(
    state: State<'_, AppState>,
    text: String,
    scope: PaletteScope,
) -> AppResult<PaletteResults> {
    super::palette_search(&state.db, text, scope).await
}

/// The filter lists that only search for text.
#[tauri::command]
pub async fn saved_searches(state: State<'_, AppState>) -> AppResult<Vec<SavedSearch>> {
    state.db.read(super::saved_searches).await
}

/// The palette's last searches, newest first.
#[tauri::command]
pub fn recent_searches(state: State<'_, AppState>) -> Vec<String> {
    state.ui_state.get().recent_searches
}

/// A search that led somewhere joins the recent ones.
#[tauri::command]
pub async fn search_remember(state: State<'_, AppState>, text: String) -> AppResult<Vec<String>> {
    state.ui_state.remember_search(text).await
}

#[tauri::command]
pub async fn search_forget(state: State<'_, AppState>, text: String) -> AppResult<Vec<String>> {
    state.ui_state.forget_search(text).await
}
