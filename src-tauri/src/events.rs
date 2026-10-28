//! Every event the backend emits to the page, with its payload type.

/// No payload; the page then calls `take_launch_events`.
pub const LAUNCH_EVENT: &str = "launch";
/// No payload; the hidden main window was shown again.
pub const WINDOW_OPENED_EVENT: &str = "window_opened";
/// Payload: the saved `Settings`.
pub const SETTINGS_CHANGED_EVENT: &str = "settings_changed";

pub fn announce_settings(app: &tauri::AppHandle, settings: &crate::store::settings::Settings) {
    use tauri::Emitter;
    if let Err(e) = app.emit(SETTINGS_CHANGED_EVENT, settings) {
        log::warn!("Couldn't announce the saved settings: {e}");
    }
}
/// Payload: the saved `Keybindings`.
pub const KEYBINDINGS_CHANGED_EVENT: &str = "keybindings_changed";
/// To the Settings window. Payload: the id of the page to show.
pub const OPEN_SETTINGS_EVENT: &str = "open_settings";
/// Payload: an `IssuesChanged`; issues in the cache changed (`db::Db::write`).
pub const ISSUES_CHANGED_EVENT: &str = "issues_changed";
/// Payload: a `LocalChanged`; parts of the owner's own setup changed.
pub const LOCAL_CHANGED_EVENT: &str = "local_changed";
/// Payload: the `SyncState` after it changed.
pub const SYNC_STATE_EVENT: &str = "sync_state";
/// Payload: a `SyncProgress`, after every page of a full sync.
pub const SYNC_PROGRESS_EVENT: &str = "sync_progress";
/// To the main window. No payload; shows or hides the sidebar.
pub const TOGGLE_SIDEBAR_EVENT: &str = "toggle_sidebar";
