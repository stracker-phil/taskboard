//! Every Tauri command the page can invoke.

use tauri::ipc::Invoke;

use crate::connection::commands as connection;
use crate::{bus, commands};

pub fn handlers() -> impl Fn(Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::get_settings,
        commands::update_settings,
        commands::get_keybindings,
        commands::set_keybindings,
        commands::get_ui_state,
        commands::set_ui_state,
        commands::open_log_folder,
        commands::open_url,
        commands::log_frontend_error,
        commands::take_launch_events,
        connection::connection_get,
        connection::connection_save,
        connection::connection_test,
        connection::projects_list,
        connection::projects_refresh,
        connection::project_track,
        connection::scope_get,
        connection::scope_validate,
        connection::scope_save,
        connection::sync_save,
        connection::cache_reset,
        crate::sync::commands::sync_overview,
        crate::sync::commands::sync_run,
        bus::app::run_command,
    ]
}
