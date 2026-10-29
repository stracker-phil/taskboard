//! The Tauri command for profile pictures.

use chrono::Utc;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;

/// The person's picture as a `data:` URL, fetched from Atlassian's avatar host when missing or stale; `None`
/// shows initials.
#[tauri::command]
pub async fn user_avatar(
    state: State<'_, AppState>,
    account_id: String,
) -> AppResult<Option<String>> {
    let client = &state.avatars;
    super::user_avatar(&state.db, account_id, Utc::now(), |url| async move {
        client.fetch(&url).await
    })
    .await
}
