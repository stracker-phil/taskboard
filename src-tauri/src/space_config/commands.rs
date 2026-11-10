//! The Tauri commands of a Space's settings page.

use tauri::State;

use super::{FieldVisibility, SpaceConfig, SpaceLayouts};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn space_config(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<SpaceConfig> {
    super::space_config(&state.db, project_id).await
}

#[tauri::command]
pub async fn space_set_layouts(
    state: State<'_, AppState>,
    project_id: String,
    layouts: SpaceLayouts,
) -> AppResult<SpaceConfig> {
    super::space_set_layouts(&state.db, project_id, layouts).await
}

/// Where the Space shows a field opted in app-wide; a place the app leaves out stays out either way.
#[tauri::command]
pub async fn space_set_field(
    state: State<'_, AppState>,
    project_id: String,
    field_id: String,
    in_list: bool,
    in_detail: bool,
) -> AppResult<SpaceConfig> {
    let visibility = FieldVisibility::from_places(in_list, in_detail);
    super::space_set_field(&state.db, project_id, field_id, visibility).await
}
