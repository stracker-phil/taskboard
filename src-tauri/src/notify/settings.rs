//! The Tauri commands of Settings → Notifications.

use tauri::{AppHandle, State};

use super::people::{PeopleField, people_fields};
use super::rules::resettle;
use crate::connection::issue_types;
use crate::error::AppResult;
use crate::events::announce_settings;
use crate::state::AppState;
use crate::store::settings::Notifications;

/// Saves the page and settles every recorded change again, so what is unread follows at once.
#[tauri::command]
pub async fn notifications_save(
    app: AppHandle,
    state: State<'_, AppState>,
    notifications: Notifications,
) -> AppResult<Notifications> {
    let tidy = notifications.tidied();
    let saved = state.settings.edit(move |s| s.notifications = tidy).await?;
    announce_settings(&app, &saved);
    resettle(&state.db, &saved).await?;
    Ok(saved.notifications)
}

/// The issue types the page offers: those of the cached issues and those left out already.
#[tauri::command]
pub async fn notification_issue_types(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    let excluded = state.settings.get().notifications.excluded_types;
    state
        .db
        .read(move |conn| issue_types(conn, &excluded))
        .await
}

/// The people fields that "named in a people field" and "Adds me to a people field" look at.
#[tauri::command]
pub async fn notification_people_fields(state: State<'_, AppState>) -> AppResult<Vec<PeopleField>> {
    state.db.read(people_fields).await
}
