//! The Tauri commands of a Space's Team settings.

use tauri::State;

use super::TeamMember;
use crate::error::AppResult;
use crate::state::AppState;
use crate::views::rows::Person;

#[tauri::command]
pub async fn space_team(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<TeamMember>> {
    let me = state.settings.get().connection.account_id;
    super::space_team(&state.db, project_id, me).await
}

#[tauri::command]
pub async fn space_set_member(
    state: State<'_, AppState>,
    project_id: String,
    account_id: String,
    active: bool,
) -> AppResult<Vec<TeamMember>> {
    let me = state.settings.get().connection.account_id;
    super::space_set_member(&state.db, project_id, account_id, active, me).await
}

/// The person chip's choices.
#[tauri::command]
pub async fn level_people(state: State<'_, AppState>, list_id: i64) -> AppResult<Vec<Person>> {
    let me = state.settings.get().connection.account_id;
    super::level_people(&state.db, list_id, me).await
}
