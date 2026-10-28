//! The Tauri commands of the view tabs.

use tauri::State;

use super::{GroupBy, Layout, Sort, View};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn views_of(state: State<'_, AppState>, list_id: i64) -> AppResult<Vec<View>> {
    super::views_of(&state.db, list_id).await
}

#[tauri::command]
pub async fn view_create(
    state: State<'_, AppState>,
    list_id: i64,
    name: String,
    layout: Layout,
) -> AppResult<View> {
    super::view_create(&state.db, list_id, name, layout).await
}

#[tauri::command]
pub async fn view_rename(state: State<'_, AppState>, id: i64, name: String) -> AppResult<View> {
    super::view_rename(&state.db, id, name).await
}

#[tauri::command]
pub async fn view_duplicate(state: State<'_, AppState>, id: i64) -> AppResult<View> {
    super::view_duplicate(&state.db, id).await
}

#[tauri::command]
pub async fn view_set_layout(
    state: State<'_, AppState>,
    id: i64,
    layout: Layout,
) -> AppResult<View> {
    super::view_set_layout(&state.db, id, layout).await
}

#[tauri::command]
pub async fn view_set_columns(
    state: State<'_, AppState>,
    id: i64,
    columns: Vec<String>,
) -> AppResult<View> {
    super::view_set_columns(&state.db, id, columns).await
}

#[tauri::command]
pub async fn view_set_group(
    state: State<'_, AppState>,
    id: i64,
    group_by: GroupBy,
) -> AppResult<View> {
    super::view_set_group(&state.db, id, group_by).await
}

#[tauri::command]
pub async fn view_set_sort(state: State<'_, AppState>, id: i64, sort: Sort) -> AppResult<View> {
    super::view_set_sort(&state.db, id, sort).await
}

#[tauri::command]
pub async fn view_move(state: State<'_, AppState>, id: i64, position: i64) -> AppResult<Vec<View>> {
    super::view_move(&state.db, id, position).await
}

/// Never the last view of a level.
#[tauri::command]
pub async fn view_delete(state: State<'_, AppState>, id: i64) -> AppResult<Vec<View>> {
    super::view_delete(&state.db, id).await
}

#[tauri::command]
pub async fn space_default_columns(
    state: State<'_, AppState>,
    project_id: String,
    columns: Vec<String>,
) -> AppResult<()> {
    super::space_default_columns(&state.db, project_id, columns).await
}
