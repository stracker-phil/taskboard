//! Every Tauri command the page can invoke.

use tauri::ipc::Invoke;

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
        bus::app::run_command,
    ]
}
