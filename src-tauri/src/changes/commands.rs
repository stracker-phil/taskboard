//! The Tauri commands for marking issues read (seen) and unread. Unread keeps what the owner reads there
//! until they leave, so the read ones are noted before the write announces the change.

use chrono::Utc;
use tauri::State;

use crate::error::AppResult;
use crate::lists::Viewer;
use crate::state::AppState;
use crate::sync::map::format_utc;

#[tauri::command]
pub async fn issues_mark_seen(state: State<'_, AppState>, ids: Vec<String>) -> AppResult<()> {
    state.nav.keep_read(&ids);
    super::mark_seen(&state.db, ids, format_utc(Utc::now())).await
}

/// Unread again: from the activity item at `from` on, or the whole issue.
#[tauri::command]
pub async fn issue_mark_unread(
    state: State<'_, AppState>,
    id: String,
    from: Option<String>,
) -> AppResult<()> {
    super::mark_unread(&state.db, id, from).await
}

/// Marks every unseen issue of the filter list seen.
#[tauri::command]
pub async fn list_mark_all_seen(state: State<'_, AppState>, list_id: i64) -> AppResult<()> {
    let viewer = Viewer::of(&state.settings.get().connection, Utc::now());
    let now = format_utc(viewer.now);
    let nav = state.nav.clone();
    let unseen = move |conn: &_| {
        let ids = crate::views::unseen_ids(conn, &viewer, list_id)?;
        nav.keep_read(&ids);
        Ok(ids)
    };
    super::mark_all_seen(&state.db, unseen, now).await?;
    Ok(())
}
