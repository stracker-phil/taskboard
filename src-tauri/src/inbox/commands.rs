//! The Tauri commands of the inbox.

use chrono::Utc;
use tauri::State;

use super::{Inbox, rows, snapshot};
use crate::error::AppResult;
use crate::state::AppState;
use crate::time::format_utc;

/// Takes a new snapshot of the unread issues and lists it.
#[tauri::command]
pub async fn inbox_open(state: State<'_, AppState>) -> AppResult<Inbox> {
    let me = state.settings.get().connection.account_id;
    let (taken, inbox) = state
        .db
        .read(move |conn| {
            let taken = snapshot(conn, me.as_deref())?;
            let inbox = rows(conn, &taken)?;
            Ok((taken, inbox))
        })
        .await?;
    state.inbox.set(Some(taken));
    Ok(inbox)
}

/// The open inbox's issues with their read state now; empty while it is closed.
#[tauri::command]
pub async fn inbox_rows(state: State<'_, AppState>) -> AppResult<Inbox> {
    let taken = state.inbox.get().unwrap_or_default();
    state.db.read(move |conn| rows(conn, &taken)).await
}

#[tauri::command]
pub fn inbox_close(state: State<'_, AppState>) {
    state.inbox.set(None);
}

#[tauri::command]
pub async fn inbox_count(state: State<'_, AppState>) -> AppResult<u32> {
    state.db.read(super::count).await
}

/// Marks every issue of the open inbox read.
#[tauri::command]
pub async fn inbox_mark_all_seen(state: State<'_, AppState>) -> AppResult<()> {
    let ids = state
        .inbox
        .get()
        .unwrap_or_default()
        .into_iter()
        .map(|snap| snap.id)
        .collect();
    crate::changes::mark_seen(&state.db, ids, format_utc(Utc::now())).await
}
