//! The Tauri commands of the view tabs.

use tauri::State;

use chrono::Utc;

use super::{BoardData, Column, GroupBy, IssueOpening, Layout, Narrowing, Sort, View, ViewData};
use crate::error::AppResult;
use crate::lists::Viewer;
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

/// Whether the List wraps long titles.
#[tauri::command]
pub async fn view_set_wrap(state: State<'_, AppState>, id: i64, wrap: bool) -> AppResult<View> {
    super::view_set_wrap(&state.db, id, wrap).await
}

#[tauri::command]
pub async fn view_set_inline_labels(
    state: State<'_, AppState>,
    id: i64,
    inline: bool,
) -> AppResult<View> {
    super::view_set_inline_labels(&state.db, id, inline).await
}

/// The tab's icon and tile color; no icon shows the layout's.
#[tauri::command]
pub async fn view_set_look(
    state: State<'_, AppState>,
    id: i64,
    icon: Option<String>,
    color: Option<String>,
) -> AppResult<View> {
    super::view_set_look(&state.db, id, icon, color).await
}

#[tauri::command]
pub async fn view_set_open_with(
    state: State<'_, AppState>,
    id: i64,
    open_with: Option<IssueOpening>,
) -> AppResult<View> {
    super::view_set_open_with(&state.db, id, open_with).await
}

#[tauri::command]
pub async fn view_set_column_collapsed(
    state: State<'_, AppState>,
    id: i64,
    status_id: String,
    collapsed: bool,
) -> AppResult<View> {
    super::view_set_column_collapsed(&state.db, id, status_id, collapsed).await
}

#[tauri::command]
pub async fn view_set_widths(
    state: State<'_, AppState>,
    id: i64,
    widths: std::collections::BTreeMap<String, i64>,
) -> AppResult<View> {
    super::view_set_widths(&state.db, id, widths).await
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

#[tauri::command]
pub async fn view_rows(
    state: State<'_, AppState>,
    list_id: i64,
    view_id: i64,
    mut narrowing: Narrowing,
) -> AppResult<ViewData> {
    narrowing.read_here = state.nav.read_here_ids();
    let viewer = Viewer::of(&state.settings.get().connection, Utc::now());
    super::view_rows(&state.db, viewer, list_id, view_id, narrowing).await
}

/// The Board layout's columns and cards.
#[tauri::command]
pub async fn view_board(
    state: State<'_, AppState>,
    list_id: i64,
    view_id: i64,
    mut narrowing: Narrowing,
) -> AppResult<BoardData> {
    narrowing.read_here = state.nav.read_here_ids();
    let viewer = Viewer::of(&state.settings.get().connection, Utc::now());
    super::view_board(&state.db, viewer, list_id, view_id, narrowing).await
}

#[tauri::command]
pub async fn columns_catalogue(state: State<'_, AppState>, list_id: i64) -> AppResult<Vec<Column>> {
    super::columns_catalogue(&state.db, list_id).await
}
