//! The app's own commands; features add theirs with their own `register`.

use tauri::{AppHandle, Manager};

use super::{Command, CommandBus};
use crate::sync::{Job, SyncEngine};
use crate::window;

type Bus = CommandBus<AppHandle>;

pub fn register(bus: &mut Bus) {
    bus.add(
        Command::new("app:settings", "Settings…")
            .help("Open Settings")
            .run_now(|app: &AppHandle| window::settings::open(app, None)),
    );
    bus.add_family(
        Command::new("app:settings:", "Settings Page")
            .help("Open Settings at a page or section")
            .hidden()
            .family("<page>")
            .run(|app, page| async move {
                window::settings::open(&app, Some(&page));
                Ok(())
            }),
    );
    bus.add_family(
        Command::new("page:", "Window Action")
            .help("Run one of the main window's own commands (the menus' items for what the page does)")
            .hidden()
            .family("<command>")
            .run(|app, id| async move {
                window::run_page_command(&app, &id);
                Ok(())
            }),
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
        Command::new("app:data", "Show Data Folder")
            .help("Show the folder with the settings and the cache in Finder")
            .run(|app| async move { crate::diagnostics::open_data_folder(app) }),
    );
    bus.add(
        Command::new("app:exportChanges", "Export Changes Sent to Jira")
            .help("Save the record of every change sent to Jira as a CSV in Downloads")
            .run(|app| async move { crate::diagnostics::export_sent_changes(app).await }),
    );
    bus.add(
        Command::new("window:close", "Close Tab or Window")
            .help("Close the tab, or hide the window with one tab or none")
            .hidden()
            .run_now(crate::nav::commands::close_tab_or_window),
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
    bus.add(
        Command::new("view:toggleBookmarksBar", "Show Bookmarks Bar")
            .help("Show the bookmarks in a bar above the content, or in the sidebar")
            .run(|app: AppHandle| async move {
                let state = app.state::<crate::state::AppState>();
                let on = !state.modes.get().bookmarks_bar;
                let modes = state
                    .modes
                    .set(crate::store::modes::Mode::BookmarksBar, on)
                    .await?;
                crate::events::announce_modes(&app, modes);
                crate::menu::place::refresh(&app);
                Ok(())
            }),
    );
    bus.add(
        Command::new("sync:refresh", "Refresh")
            .help("Fetch what changed in Jira")
            .run_now(|app: &AppHandle| app.state::<SyncEngine>().queue(Job::Delta)),
    );
    bus.add(
        Command::new("sync:full", "Full Sync")
            .help("Fetch every issue of the sync scope again")
            .run_now(|app: &AppHandle| app.state::<SyncEngine>().queue(Job::Full)),
    );
    bus.add(
        Command::new("app:compactCache", "Compact Cache")
            .help("Rewrite the Jira cache without the space that deleted issues left")
            .run(|app: AppHandle| async move {
                let db = &app.state::<crate::state::AppState>().db;
                if let Some(size) = db.compact_cache().await? {
                    log::info!("Compacted the cache to {} bytes", size.bytes);
                }
                Ok(())
            }),
    );
    #[cfg(debug_assertions)]
    bus.add(
        Command::new("app:reload", "Reload")
            .help("Reload the focused window's page")
            .run_now(window::reload_focused),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registered() -> Bus {
        let mut bus = Bus::default();
        register(&mut bus);
        bus
    }

    #[test]
    fn settings_page_ids_resolve_to_the_hidden_family() {
        let bus = registered();

        for id in [
            "app:settings:projects",
            "app:settings:general",
            "app:settings:projects:sync",
        ] {
            assert!(bus.knows(id), "{id}");
            let info = bus.info(id).unwrap();
            assert_eq!(info.title, "Settings Page", "{id}");
            assert_eq!(info.help, "Open Settings at a page or section", "{id}");
            assert!(info.hidden, "{id}");
        }
    }

    #[test]
    fn the_bookmarks_bar_toggle_is_a_registered_command() {
        let bus = registered();

        let info = bus.info("view:toggleBookmarksBar").unwrap();
        assert_eq!(info.title, "Show Bookmarks Bar");
        assert!(!info.hidden);
    }

    #[test]
    fn the_settings_page_family_takes_a_page_argument() {
        let bus = registered();
        assert_eq!(bus.family("app:settings:").unwrap().arg, "<page>");
    }

    #[test]
    fn page_command_ids_resolve_to_the_hidden_window_action_family() {
        let bus = registered();

        for id in [
            "page:toolbar.me",
            "page:view.layout.board",
            "page:issue.copyKey",
        ] {
            assert!(bus.knows(id), "{id}");
            let info = bus.info(id).unwrap();
            assert_eq!(info.title, "Window Action", "{id}");
            assert_eq!(
                info.help,
                "Run one of the main window's own commands (the menus' items for what the page does)",
                "{id}"
            );
            assert!(info.hidden, "{id}");
        }
    }

    #[test]
    fn the_page_family_takes_a_command_argument() {
        let bus = registered();
        assert_eq!(bus.family("page:").unwrap().arg, "<command>");
    }

    #[test]
    fn plain_settings_stays_its_own_visible_command() {
        let bus = registered();

        let info = bus.info("app:settings").unwrap();
        assert_eq!(info.title, "Settings…");
        assert!(!info.hidden);
        assert!(bus.family("app:settings").is_none());
    }

    #[test]
    fn both_sync_commands_are_registered_visible_with_their_titles() {
        let bus = registered();

        for (id, title) in [("sync:refresh", "Refresh"), ("sync:full", "Full Sync")] {
            assert!(bus.knows(id), "{id}");
            let info = bus.info(id).unwrap();
            assert_eq!(info.title, title, "{id}");
            assert!(!info.hidden, "{id}");
        }
    }

    #[test]
    fn show_data_folder_is_registered_and_visible() {
        let bus = registered();

        assert!(bus.knows("app:data"));
        let info = bus.info("app:data").unwrap();
        assert_eq!(info.title, "Show Data Folder");
        assert!(!info.hidden);
    }

    #[test]
    fn compact_cache_is_registered_and_visible() {
        let bus = registered();

        assert!(bus.knows("app:compactCache"));
        let info = bus.info("app:compactCache").unwrap();
        assert_eq!(info.title, "Compact Cache");
        assert!(!info.hidden);
    }

    #[test]
    fn export_changes_is_registered_and_visible() {
        let bus = registered();

        assert!(bus.knows("app:exportChanges"));
        let info = bus.info("app:exportChanges").unwrap();
        assert_eq!(info.title, "Export Changes Sent to Jira");
        assert!(!info.hidden);
    }

    #[test]
    fn the_settings_page_family_is_the_only_hidden_settings_entry() {
        let bus = registered();

        let hidden: Vec<&str> = bus
            .list()
            .filter(|listed| listed.info().hidden)
            .map(|listed| listed.id())
            .filter(|id| id.starts_with("app:settings"))
            .collect();
        assert_eq!(hidden, ["app:settings:"]);
    }
}
