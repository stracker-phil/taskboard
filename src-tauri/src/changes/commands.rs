//! The Tauri commands for marking issues seen.

use chrono::Utc;
use tauri::State;

use crate::error::AppResult;
use crate::lists::Viewer;
use crate::state::AppState;
use crate::sync::map::format_utc;

#[tauri::command]
pub async fn issues_mark_seen(state: State<'_, AppState>, ids: Vec<String>) -> AppResult<()> {
    super::mark_seen(&state.db, ids, format_utc(Utc::now())).await
}

/// Marks every unseen issue of the filter list seen.
#[tauri::command]
pub async fn list_mark_all_seen(state: State<'_, AppState>, list_id: i64) -> AppResult<()> {
    let viewer = Viewer::of(&state.settings.get().connection, Utc::now());
    let now = format_utc(viewer.now);
    super::mark_all_seen(
        &state.db,
        move |conn| crate::views::unseen_ids(conn, &viewer, list_id),
        now,
    )
    .await
}
