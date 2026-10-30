//! The Tauri command for the command palette.

use tauri::State;

use super::{PaletteResults, PaletteScope};
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
