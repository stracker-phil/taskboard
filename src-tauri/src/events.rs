//! Every event the backend emits to the page, with its payload type.

/// No payload; the page then calls `take_launch_events`.
pub const LAUNCH_EVENT: &str = "launch";
/// No payload; the hidden main window was shown again.
pub const WINDOW_OPENED_EVENT: &str = "window_opened";
/// Payload: the saved `Settings`.
pub const SETTINGS_CHANGED_EVENT: &str = "settings_changed";
/// Payload: the saved `Keybindings`.
pub const KEYBINDINGS_CHANGED_EVENT: &str = "keybindings_changed";
/// To the Settings window. Payload: the id of the page to show.
pub const OPEN_SETTINGS_EVENT: &str = "open_settings";
/// To the main window. No payload; shows or hides the sidebar.
pub const TOGGLE_SIDEBAR_EVENT: &str = "toggle_sidebar";
