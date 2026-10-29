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

#[tauri::command]
pub async fn space_set_field(
    state: State<'_, AppState>,
    project_id: String,
    field_id: String,
    visibility: FieldVisibility,
) -> AppResult<SpaceConfig> {
    super::space_set_field(&state.db, project_id, field_id, visibility).await
}
