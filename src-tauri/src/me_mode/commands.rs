//! The Tauri commands of Me mode's settings.

use tauri::{AppHandle, State};

use super::{MeRuleEdit, UserField, edited};
use crate::error::AppResult;
use crate::events::announce_settings;
use crate::state::AppState;
use crate::store::settings::Settings;

/// The fields Settings → Me Mode may add to the Me popup.
#[tauri::command]
pub async fn me_user_fields(state: State<'_, AppState>) -> AppResult<Vec<UserField>> {
    state.db.read(super::user_fields).await
}

/// Edits the Me rules in the settings; every view refreshes.
#[tauri::command]
pub async fn me_rule_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    edit: MeRuleEdit,
) -> AppResult<Settings> {
    let saved = state
        .settings
        .edit(move |s| {
            s.behaviour.me_rules = edited(std::mem::take(&mut s.behaviour.me_rules), edit)
        })
        .await?;
    announce_settings(&app, &saved);
    Ok(saved)
}
