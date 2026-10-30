//! The Tauri commands of the Connection and Sync settings pages.

use chrono::{Local, NaiveDate};
use tauri::{AppHandle, State};

use super::{ConnectionInput, ConnectionTest, ConnectionView, ProjectRow, ScopeView};
use crate::error::AppResult;
use crate::events::announce_settings;
use crate::state::AppState;
use crate::store::settings::SyncSettings;
use crate::sync::SyncEngine;

fn today() -> NaiveDate {
    Local::now().date_naive()
}

#[tauri::command]
pub fn connection_get(state: State<'_, AppState>) -> ConnectionView {
    super::view(&state.settings, &state.jira, today())
}

/// `token` is stored in the Keychain and never returned.
#[tauri::command]
pub async fn connection_save(
    app: AppHandle,
    state: State<'_, AppState>,
    sync: State<'_, SyncEngine>,
    input: ConnectionInput,
    token: Option<String>,
) -> AppResult<ConnectionView> {
    let saved = super::save(&state.settings, &state.jira, input, token, today()).await?;
    sync.connection_changed();
    announce_settings(&app, &state.settings.get());
    log::info!("Jira connection saved");
    Ok(saved)
}

#[tauri::command]
pub async fn connection_test(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<ConnectionTest> {
    let tested = super::test(&state.settings, &state.jira).await;
    // Even a failed test may have found the cloud id.
    announce_settings(&app, &state.settings.get());
    tested
}

#[tauri::command]
pub async fn projects_list(state: State<'_, AppState>) -> AppResult<Vec<ProjectRow>> {
    super::list_projects(&state.db).await
}

#[tauri::command]
pub async fn projects_refresh(state: State<'_, AppState>) -> AppResult<Vec<ProjectRow>> {
    let client = state.jira.client(&state.settings.get().connection)?;
    super::refresh_projects(&state.db, &client).await
}

#[tauri::command]
pub async fn project_track(
    state: State<'_, AppState>,
    project_id: String,
    tracked: bool,
) -> AppResult<()> {
    if tracked && let Ok(client) = state.jira.client(&state.settings.get().connection) {
        // Without the versions the Space just has no Release list.
        if let Err(e) = super::fetch_versions(&state.db, &client, project_id.clone()).await {
            log::warn!("Couldn't fetch the project's versions: {e}");
        }
    }
    super::track_project(&state.db, project_id, tracked).await
}

#[tauri::command]
pub async fn scope_get(state: State<'_, AppState>) -> AppResult<ScopeView> {
    super::scope_view(&state.db, &state.settings.get().sync).await
}

/// Jira's messages come back as an `invalid_jql` error.
#[tauri::command]
pub async fn scope_validate(state: State<'_, AppState>, jql: String) -> AppResult<()> {
    let client = state.jira.client(&state.settings.get().connection)?;
    super::validate_scope(&client, &jql).await
}

/// `jql: null` goes back to the default scope.
#[tauri::command]
pub async fn scope_save(
    app: AppHandle,
    state: State<'_, AppState>,
    jql: Option<String>,
) -> AppResult<ScopeView> {
    let client = state.jira.client(&state.settings.get().connection).ok();
    let sync = super::save_scope(&state.settings, client.as_deref(), jql).await?;
    announce_settings(&app, &state.settings.get());
    super::scope_view(&state.db, &sync).await
}

#[tauri::command]
pub async fn sync_save(
    app: AppHandle,
    state: State<'_, AppState>,
    sync: SyncSettings,
) -> AppResult<SyncSettings> {
    let saved = super::save_intervals(&state.settings, sync).await?;
    announce_settings(&app, &state.settings.get());
    Ok(saved)
}

/// Deletes the Jira mirror; the next sync is a full one. Spaces, filter lists, views and pins stay.
#[tauri::command]
pub async fn cache_reset(state: State<'_, AppState>) -> AppResult<()> {
    state.db.reset_cache().await?;
    log::info!("Cache reset");
    Ok(())
}
