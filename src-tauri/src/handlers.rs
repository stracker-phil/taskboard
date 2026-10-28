//! Every Tauri command the page can invoke.

use tauri::ipc::Invoke;

use crate::connection::commands as connection;
use crate::lists::commands as lists;
use crate::views::commands as views;
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
        lists::sidebar_tree,
        lists::list_create,
        lists::list_rename,
        lists::list_duplicate,
        lists::list_delete,
        lists::list_move,
        lists::list_set_filter,
        lists::list_match_count,
        lists::space_move,
        lists::space_update,
        views::views_of,
        views::view_create,
        views::view_rename,
        views::view_duplicate,
        views::view_set_layout,
        views::view_set_columns,
        views::view_set_group,
        views::view_set_sort,
        views::view_move,
        views::view_delete,
        views::space_default_columns,
        bus::app::run_command,
    ]
}
