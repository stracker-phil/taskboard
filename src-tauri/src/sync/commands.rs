//! The Tauri commands of the sync: the overview Settings shows and "Sync now".

use tauri::State;

use super::{Job, SyncEngine, SyncOverview, overview};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub async fn sync_overview(
    state: State<'_, AppState>,
    sync: State<'_, SyncEngine>,
) -> AppResult<SyncOverview> {
    overview(&state.db, sync.state()).await
}

/// Queues a full sync and returns at once; progress arrives as `sync_progress` and `sync_state`.
#[tauri::command]
pub fn sync_run(sync: State<'_, SyncEngine>) {
    sync.queue(Job::Full);
}
