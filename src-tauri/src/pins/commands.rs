//! The Tauri commands for pins.

use chrono::Utc;
use tauri::State;

use super::{PinDuration, PinKind, Pins};
use crate::error::AppResult;
use crate::lists::Viewer;
use crate::state::AppState;

fn viewer(state: &AppState) -> Viewer {
    Viewer::of(&state.settings.get().connection, Utc::now())
}

#[tauri::command]
pub async fn pins_list(state: State<'_, AppState>) -> AppResult<Pins> {
    super::pins_list(&state.db, viewer(&state)).await
}

/// Pins the target, or changes the duration and reason of its pin.
#[tauri::command]
pub async fn pin_create(
    state: State<'_, AppState>,
    kind: PinKind,
    target: String,
    duration: PinDuration,
    reason: Option<String>,
) -> AppResult<()> {
    super::pin_create(&state.db, viewer(&state), kind, target, duration, reason).await
}

#[tauri::command]
pub async fn pin_remove(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    super::pin_remove(&state.db, viewer(&state), id).await
}

#[tauri::command]
pub async fn pin_toggle_issue(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let duration = state.settings.get().behaviour.pin_default;
    super::pin_toggle_issue(&state.db, viewer(&state), id, duration).await
}
