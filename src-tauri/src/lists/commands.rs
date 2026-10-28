//! The Tauri commands of the sidebar tree.

use chrono::Utc;
use tauri::State;

use super::{MatchCount, SidebarTree, SpaceUpdate, Viewer};
use crate::error::AppResult;
use crate::filter::Filter;
use crate::state::AppState;

fn viewer(state: &AppState) -> Viewer {
    Viewer::of(&state.settings.get().connection, Utc::now())
}

#[tauri::command]
pub async fn sidebar_tree(state: State<'_, AppState>) -> AppResult<SidebarTree> {
    super::sidebar_tree(&state.db, viewer(&state)).await
}

/// `parent_id`: a Space's node, or Everything's for a list under Home. Returns the new list's id.
#[tauri::command]
pub async fn list_create(
    state: State<'_, AppState>,
    parent_id: i64,
    name: String,
    filter: Option<Filter>,
) -> AppResult<i64> {
    super::list_create(&state.db, viewer(&state), parent_id, name, filter).await
}

#[tauri::command]
pub async fn list_rename(state: State<'_, AppState>, id: i64, name: String) -> AppResult<()> {
    super::list_rename(&state.db, id, name).await
}

#[tauri::command]
pub async fn list_duplicate(state: State<'_, AppState>, id: i64) -> AppResult<i64> {
    super::list_duplicate(&state.db, id).await
}

#[tauri::command]
pub async fn list_delete(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    super::list_delete(&state.db, id).await
}

#[tauri::command]
pub async fn list_move(state: State<'_, AppState>, id: i64, position: i64) -> AppResult<()> {
    super::list_move(&state.db, id, position).await
}

#[tauri::command]
pub async fn list_set_filter(state: State<'_, AppState>, id: i64, filter: Filter) -> AppResult<()> {
    super::list_set_filter(&state.db, viewer(&state), id, filter).await
}

#[tauri::command]
pub async fn list_match_count(
    state: State<'_, AppState>,
    id: i64,
    filter: Filter,
) -> AppResult<MatchCount> {
    super::list_match_count(&state.db, viewer(&state), id, filter).await
}

#[tauri::command]
pub async fn space_move(
    state: State<'_, AppState>,
    project_id: String,
    position: i64,
) -> AppResult<()> {
    super::space_move(&state.db, project_id, position).await
}

#[tauri::command]
pub async fn space_update(
    state: State<'_, AppState>,
    project_id: String,
    update: SpaceUpdate,
) -> AppResult<()> {
    super::space_update(&state.db, project_id, update).await
}
