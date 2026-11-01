//! The Tauri commands of Settings → Team.

use tauri::State;

use super::TeamMatrix;
use crate::error::AppResult;
use crate::state::AppState;
use crate::views::rows::Person;

#[tauri::command]
pub async fn team_matrix(state: State<'_, AppState>) -> AppResult<TeamMatrix> {
    let me = state.settings.get().connection.account_id;
    super::team_matrix_of(&state.db, me).await
}

#[tauri::command]
pub async fn space_set_member(
    state: State<'_, AppState>,
    project_id: String,
    account_id: String,
    active: bool,
) -> AppResult<TeamMatrix> {
    let me = state.settings.get().connection.account_id;
    super::space_set_member(&state.db, project_id, account_id, active, me).await
}

/// The person chip's choices.
#[tauri::command]
pub async fn level_people(state: State<'_, AppState>, list_id: i64) -> AppResult<Vec<Person>> {
    let me = super::Me::of(&state.settings.get().connection);
    super::level_people(&state.db, list_id, me).await
}
