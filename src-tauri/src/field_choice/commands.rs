//! The Tauri commands of Settings → Custom fields.

use tauri::State;

use super::FieldChoiceRow;
use crate::error::AppResult;
use crate::state::AppState;
use crate::sync::{Job, SyncEngine};

#[tauri::command]
pub async fn field_choices(state: State<'_, AppState>) -> AppResult<Vec<FieldChoiceRow>> {
    super::field_choices(&state.db).await
}

#[tauri::command]
pub async fn field_choice_set(
    state: State<'_, AppState>,
    sync: State<'_, SyncEngine>,
    field_id: String,
    in_list: bool,
    in_detail: bool,
) -> AppResult<Vec<FieldChoiceRow>> {
    let saved = super::field_choice_set(&state.db, field_id, in_list, in_detail).await?;
    if saved.newly_fetched {
        sync.queue(Job::Full);
    }
    Ok(saved.rows)
}
