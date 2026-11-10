//! The Tauri commands of the view tabs.

use tauri::State;

use chrono::Utc;

use super::{
    BoardData, Column, GroupBy, IssueOpening, Layout, Narrowing, Sort, SubtaskMode, View, ViewData,
};
use crate::error::AppResult;
use crate::filter::Filter;
use crate::lists::Viewer;
use crate::me_mode::MeMatch;
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

/// How the view shows subtasks; the Board ignores it.
#[tauri::command]
pub async fn view_set_subtasks(
    state: State<'_, AppState>,
    id: i64,
    mode: SubtaskMode,
) -> AppResult<View> {
    super::view_set_subtasks(&state.db, id, mode).await
}

/// The view's own conditions, ANDed with its list's.
#[tauri::command]
pub async fn view_set_filter(
    state: State<'_, AppState>,
    id: i64,
    filter: Filter,
) -> AppResult<View> {
    let viewer = Viewer::of(&state.settings.get().connection, Utc::now());
    super::view_set_filter(&state.db, viewer, id, filter).await
}

/// The Me rules the view has on; none follows the app's.
#[tauri::command]
pub async fn view_set_me_rules(
    state: State<'_, AppState>,
    id: i64,
    rules: Option<Vec<MeMatch>>,
) -> AppResult<View> {
    super::view_set_me_rules(&state.db, id, rules).await
}

#[tauri::command]
pub async fn view_set_only_mine(state: State<'_, AppState>, id: i64, on: bool) -> AppResult<View> {
    super::view_set_only_mine(&state.db, id, on).await
}

#[tauri::command]
pub async fn view_set_person(
    state: State<'_, AppState>,
    id: i64,
    person: Option<String>,
) -> AppResult<View> {
    super::view_set_person(&state.db, id, person).await
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

/// The page's narrowing with what Rust adds: Me mode's rules and the app-wide modes.
fn with_modes(state: &AppState, narrowing: Narrowing) -> (Narrowing, Viewer) {
    let settings = state.settings.get();
    let modes = state.modes.get();
    let narrowing = Narrowing {
        me_rules: narrowing.resolve_me_rules(&settings.behaviour.me_rules),
        me: narrowing.me || modes.me_everywhere,
        ..narrowing
    };
    (narrowing, Viewer::of(&settings.connection, Utc::now()))
}

#[tauri::command]
pub async fn view_rows(
    state: State<'_, AppState>,
    list_id: i64,
    view_id: i64,
    narrowing: Narrowing,
) -> AppResult<ViewData> {
    let (narrowing, viewer) = with_modes(&state, narrowing);
    super::view_rows(&state.db, viewer, list_id, view_id, narrowing).await
}

/// The person chip's choices for the view as it shows now.
#[tauri::command]
pub async fn view_people(
    state: State<'_, AppState>,
    list_id: i64,
    view_id: i64,
    narrowing: Narrowing,
) -> AppResult<Vec<crate::views::rows::Person>> {
    let me = crate::team::Me::of(&state.settings.get().connection);
    let (narrowing, viewer) = with_modes(&state, narrowing);
    super::view_people(&state.db, viewer, me, list_id, view_id, narrowing).await
}

/// The Board layout's columns and cards.
#[tauri::command]
pub async fn view_board(
    state: State<'_, AppState>,
    list_id: i64,
    view_id: i64,
    narrowing: Narrowing,
) -> AppResult<BoardData> {
    let (narrowing, viewer) = with_modes(&state, narrowing);
    super::view_board(&state.db, viewer, list_id, view_id, narrowing).await
}

#[tauri::command]
pub async fn columns_catalogue(state: State<'_, AppState>, list_id: i64) -> AppResult<Vec<Column>> {
    super::columns_catalogue(&state.db, list_id).await
}
