//! The app's own commands; features add theirs with their own `register`.

use tauri::AppHandle;

use super::{Command, CommandBus};
use crate::window;

type Bus = CommandBus<AppHandle>;

pub fn register(bus: &mut Bus) {
    bus.add(
        Command::new("app:settings", "Settings…")
            .help("Open Settings")
            .run_now(|app: &AppHandle| window::settings::open(app, None)),
    );
    bus.add(
        Command::new("app:quit", "Quit Taskboard")
            .quits()
            .run_now(|app: &AppHandle| app.exit(0)),
    );
    bus.add(
        Command::new("app:logs", "Open Log Folder")
            .run(|app| async move { crate::commands::open_log_folder(app) }),
    );
    bus.add(
        Command::new("window:close", "Close Window")
            .help("Hide the focused window")
            .hidden()
            .run_now(window::close_focused),
    );
    bus.add(
        Command::new("app:show", "Show Taskboard")
            .help("Show the Taskboard window")
            .run_now(window::show_main),
    );
    bus.add(
        Command::new("view:toggleSidebar", "Toggle Sidebar")
            .help("Show or hide the sidebar")
            .run_now(window::toggle_sidebar),
    );
    #[cfg(debug_assertions)]
    bus.add(
        Command::new("app:reload", "Reload")
            .help("Reload the focused window's page")
            .run_now(window::reload_focused),
    );
}
