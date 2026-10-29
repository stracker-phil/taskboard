//! The app menu (Taskboard, File, Edit, Go, List, Issue, View, Window, Help). Every custom item's id is the command
//! it runs; `page:<id>` items run the main window's own commands (the same ids its keys run). The standard
//! items (About, Services, Hide, the Edit items, Minimize) are macOS's own. Plain-letter keys (J, K, P, M) stay
//! off the menu items, so they never take a typed letter from a text field.

use tauri::menu::{AboutMetadata, Menu, MenuItem, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::bus::{self, CommandStatus};

/// A menu item that runs a command, with its key equivalent in Tauri's accelerator notation.
pub struct Item {
    pub label: &'static str,
    pub command: &'static str,
    pub accelerator: Option<&'static str>,
}

const fn item(label: &'static str, command: &'static str, accelerator: &'static str) -> Item {
    Item {
        label,
        command,
        accelerator: Some(accelerator),
    }
}

const fn plain(label: &'static str, command: &'static str) -> Item {
    Item {
        label,
        command,
        accelerator: None,
    }
}

pub const SETTINGS: Item = item("Settings…", "app:settings", "CmdOrCtrl+,");
pub const QUIT: Item = item("Quit Taskboard", "app:quit", "CmdOrCtrl+Q");
pub const FILE: &[Item] = &[
    item("New Filter List", "page:list.new", "CmdOrCtrl+N"),
    item("New View", "page:view.new", "CmdOrCtrl+T"),
];
pub const FILE_SYNC: &[Item] = &[
    item("Refresh", "sync:refresh", "CmdOrCtrl+R"),
    item("Full Sync", "sync:full", "CmdOrCtrl+Shift+R"),
];
/// Opens Settings at the Cache group, which asks before it empties the cache.
pub const RESET_CACHE: Item = plain("Reset Cache…", "app:settings:cache");
pub const CLOSE: Item = item("Close Window", "window:close", "CmdOrCtrl+W");
pub const GO_LEVELS: &[Item] = &[
    item("Command Palette…", "page:palette.open", "CmdOrCtrl+K"),
    item("Everything", "page:go.everything", "CmdOrCtrl+Shift+E"),
    item("Unread", "page:go.recent", "CmdOrCtrl+Shift+U"),
    item("Bookmarks", "page:go.pinned", "CmdOrCtrl+Shift+P"),
];
pub const GO_MOVES: &[Item] = &[
    item("Previous Space", "nav:previousSpace", "CmdOrCtrl+Alt+Up"),
    item("Next Space", "nav:nextSpace", "CmdOrCtrl+Alt+Down"),
    item("Previous View", "page:view.previous", "Ctrl+Shift+Tab"),
    item("Next View", "page:view.next", "Ctrl+Tab"),
];
pub const LIST_LAYOUTS: &[Item] = &[
    item("Show as List", "page:view.layout.list", "CmdOrCtrl+Ctrl+1"),
    item(
        "Show as Board",
        "page:view.layout.board",
        "CmdOrCtrl+Ctrl+2",
    ),
    item(
        "Show as Table",
        "page:view.layout.table",
        "CmdOrCtrl+Ctrl+3",
    ),
];
pub const LIST_TOOLBAR: &[Item] = &[
    item("Group and Sort…", "page:toolbar.group", "CmdOrCtrl+Alt+G"),
    item("Columns…", "page:toolbar.columns", "CmdOrCtrl+Alt+C"),
    item("Edit Filter…", "page:toolbar.filter", "CmdOrCtrl+Alt+F"),
    item("Only My Issues", "page:toolbar.me", "CmdOrCtrl+Shift+M"),
    item("Filter This View", "page:toolbar.search", "CmdOrCtrl+F"),
];
pub const LIST_SEEN: &[Item] = &[item(
    "Mark All as Read",
    "page:recent.markAllSeen",
    "CmdOrCtrl+Shift+A",
)];
pub const ISSUE: &[Item] = &[
    plain("Open", "page:issue.open"),
    plain("Open or Close Peek", "page:issue.togglePeek"),
    plain("Open Epic", "page:issue.openEpic"),
    item("Open in Jira", "page:issue.openInJira", "CmdOrCtrl+Enter"),
    plain("Copy Key", "page:issue.copyKey"),
    item("Copy Link", "page:issue.copyUrl", "CmdOrCtrl+Shift+C"),
    plain("Add or Remove Bookmark", "page:issue.togglePin"),
    plain("Mark as Read or Unread", "page:issue.markSeen"),
];
/// The window's appearance only; what the page shows lives in Go, List and Issue.
pub const VIEW: &[Item] = &[item(
    "Toggle Sidebar",
    "view:toggleSidebar",
    "CmdOrCtrl+Backslash",
)];
pub const RELOAD: Item = item("Reload", "app:reload", "CmdOrCtrl+Alt+R");
pub const HELP: &[Item] = &[
    plain("Open Log Folder", "app:logs"),
    plain("Show Data Folder", "app:data"),
];

fn build_item(
    app: &AppHandle,
    commands: &dyn CommandStatus,
    item: &Item,
) -> tauri::Result<Option<MenuItem<Wry>>> {
    if !commands.knows(item.command) {
        return Ok(None);
    }
    let mut builder = MenuItemBuilder::with_id(item.command, item.label)
        .enabled(commands.is_enabled(item.command));
    if let Some(accelerator) = item.accelerator {
        builder = builder.accelerator(accelerator);
    }
    builder.build(app).map(Some)
}

fn with_items<'a>(
    app: &'a AppHandle,
    commands: &dyn CommandStatus,
    mut builder: SubmenuBuilder<'a, Wry, AppHandle>,
    items: &[Item],
) -> tauri::Result<SubmenuBuilder<'a, Wry, AppHandle>> {
    for item in items {
        if let Some(built) = build_item(app, commands, item)? {
            builder = builder.item(&built);
        }
    }
    Ok(builder)
}

fn app_submenu(app: &AppHandle, commands: &dyn CommandStatus) -> tauri::Result<Submenu<Wry>> {
    let builder = SubmenuBuilder::new(app, crate::APP_NAME)
        .about(Some(AboutMetadata::default()))
        .separator();
    let builder = with_items(app, commands, builder, &[SETTINGS])?
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator();
    with_items(app, commands, builder, &[QUIT])?.build()
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let registry = app.state::<bus::Commands>();
    let commands = registry.available(app);
    let taskboard = app_submenu(app, &commands)?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let file = with_items(app, &commands, SubmenuBuilder::new(app, "File"), FILE)?.separator();
    let file = with_items(app, &commands, file, FILE_SYNC)?.separator();
    let file = with_items(app, &commands, file, &[RESET_CACHE])?.separator();
    let file = with_items(app, &commands, file, &[CLOSE])?.build()?;
    let go = with_items(app, &commands, SubmenuBuilder::new(app, "Go"), GO_LEVELS)?.separator();
    let go = with_items(app, &commands, go, GO_MOVES)?.build()?;
    let list = with_items(
        app,
        &commands,
        SubmenuBuilder::new(app, "List"),
        LIST_LAYOUTS,
    )?
    .separator();
    let list = with_items(app, &commands, list, LIST_TOOLBAR)?.separator();
    let list = with_items(app, &commands, list, LIST_SEEN)?.build()?;
    let issue = with_items(app, &commands, SubmenuBuilder::new(app, "Issue"), ISSUE)?.build()?;
    let view = with_items(app, &commands, SubmenuBuilder::new(app, "View"), VIEW)?;
    let view = with_items(app, &commands, view, &[RELOAD])?
        .separator()
        .fullscreen()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .bring_all_to_front()
        .build()?;
    let help = with_items(app, &commands, SubmenuBuilder::new(app, "Help"), HELP)?.build()?;
    Menu::with_items(
        app,
        &[
            &taskboard, &file, &edit, &go, &list, &issue, &view, &window, &help,
        ],
    )
}

pub fn clicked(app: &AppHandle, id: &str) {
    if app.state::<bus::Commands>().knows(id) {
        bus::spawn(app, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::CommandBus;

    fn registered() -> CommandBus<AppHandle> {
        let mut bus = CommandBus::<AppHandle>::default();
        bus::register(&mut bus);
        crate::nav::commands::register(&mut bus);
        bus
    }

    fn all_items() -> Vec<&'static Item> {
        let mut items: Vec<&Item> = vec![&SETTINGS, &QUIT, &RESET_CACHE, &CLOSE, &RELOAD];
        for list in [
            FILE,
            FILE_SYNC,
            GO_LEVELS,
            GO_MOVES,
            LIST_LAYOUTS,
            LIST_TOOLBAR,
            LIST_SEEN,
            ISSUE,
            VIEW,
            HELP,
        ] {
            items.extend(list);
        }
        items
    }

    #[test]
    fn every_menu_item_runs_a_registered_command() {
        let bus = registered();
        let mut items = all_items();
        if !cfg!(debug_assertions) {
            items.retain(|item| item.command != RELOAD.command);
        }
        for item in items {
            assert!(bus.knows(item.command), "{}", item.command);
        }
    }

    #[test]
    fn page_items_use_the_ids_the_page_registers() {
        let page_ids: Vec<&str> = [
            FILE,
            GO_LEVELS,
            GO_MOVES,
            LIST_LAYOUTS,
            LIST_TOOLBAR,
            LIST_SEEN,
            ISSUE,
        ]
        .into_iter()
        .flatten()
        .filter_map(|item| item.command.strip_prefix("page:"))
        .collect();
        assert_eq!(
            page_ids,
            [
                "list.new",
                "view.new",
                "palette.open",
                "go.everything",
                "go.recent",
                "go.pinned",
                "view.previous",
                "view.next",
                "view.layout.list",
                "view.layout.board",
                "view.layout.table",
                "toolbar.group",
                "toolbar.columns",
                "toolbar.filter",
                "toolbar.me",
                "toolbar.search",
                "recent.markAllSeen",
                "issue.open",
                "issue.togglePeek",
                "issue.openEpic",
                "issue.openInJira",
                "issue.copyKey",
                "issue.copyUrl",
                "issue.togglePin",
                "issue.markSeen",
            ]
        );
    }

    #[test]
    fn the_space_moves_run_the_nav_commands() {
        let commands: Vec<&str> = GO_MOVES
            .iter()
            .filter(|item| !item.command.starts_with("page:"))
            .map(|item| item.command)
            .collect();
        assert_eq!(commands, ["nav:previousSpace", "nav:nextSpace"]);
    }

    #[test]
    fn no_item_binds_a_plain_letter_or_the_copy_key() {
        for item in all_items() {
            let Some(accelerator) = item.accelerator else {
                continue;
            };
            assert_ne!(accelerator, "CmdOrCtrl+C", "{}", item.command);
            let last = accelerator.rsplit('+').next().unwrap_or(accelerator);
            let plain_letter = last.len() == 1
                && last.chars().all(|c| c.is_ascii_alphabetic())
                && accelerator == last;
            assert!(!plain_letter, "{}", item.command);
        }
        let copy_key = ISSUE
            .iter()
            .find(|item| item.command == "page:issue.copyKey")
            .unwrap();
        assert_eq!(copy_key.accelerator, None);
    }

    #[test]
    fn reload_is_registered_only_in_debug_builds() {
        assert_eq!(registered().knows(RELOAD.command), cfg!(debug_assertions));
    }

    #[test]
    fn the_key_equivalents_follow_the_mac_conventions() {
        assert_eq!(SETTINGS.accelerator, Some("CmdOrCtrl+,"));
        assert_eq!(QUIT.accelerator, Some("CmdOrCtrl+Q"));
        assert_eq!(CLOSE.accelerator, Some("CmdOrCtrl+W"));
        let toggle = VIEW
            .iter()
            .find(|item| item.command == "view:toggleSidebar")
            .unwrap();
        assert_eq!(toggle.label, "Toggle Sidebar");
        assert_eq!(toggle.accelerator, Some("CmdOrCtrl+Backslash"));
    }

    #[test]
    fn the_view_menu_holds_only_window_appearance() {
        let found: Vec<(&str, &str, Option<&str>)> = VIEW
            .iter()
            .map(|item| (item.label, item.command, item.accelerator))
            .collect();
        assert_eq!(
            found,
            [(
                "Toggle Sidebar",
                "view:toggleSidebar",
                Some("CmdOrCtrl+Backslash")
            )]
        );
        for item in VIEW {
            assert!(!item.command.starts_with("page:"), "{}", item.command);
            assert!(!item.command.starts_with("sync:"), "{}", item.command);
        }
        assert_eq!(RELOAD.accelerator, Some("CmdOrCtrl+Alt+R"));
    }

    #[test]
    fn the_file_menu_holds_refresh_full_sync_and_close_window() {
        let sync: Vec<(&str, &str, Option<&str>)> = FILE_SYNC
            .iter()
            .map(|item| (item.label, item.command, item.accelerator))
            .collect();
        assert_eq!(
            sync,
            [
                ("Refresh", "sync:refresh", Some("CmdOrCtrl+R")),
                ("Full Sync", "sync:full", Some("CmdOrCtrl+Shift+R")),
            ]
        );
        assert_eq!(CLOSE.label, "Close Window");
        assert_eq!(CLOSE.command, "window:close");
    }

    #[test]
    fn mark_all_seen_lives_in_the_list_menu_not_in_issue() {
        fn commands(items: &[Item]) -> Vec<&str> {
            items.iter().map(|item| item.command).collect()
        }
        assert_eq!(commands(LIST_SEEN), ["page:recent.markAllSeen"]);
        assert!(!commands(ISSUE).contains(&"page:recent.markAllSeen"));
    }

    #[test]
    fn unread_and_bookmark_items_carry_their_labels() {
        let label_of = |items: &[Item], command: &str| {
            items
                .iter()
                .find(|item| item.command == command)
                .map(|item| item.label)
        };
        assert_eq!(label_of(GO_LEVELS, "page:go.recent"), Some("Unread"));
        assert_eq!(label_of(GO_LEVELS, "page:go.pinned"), Some("Bookmarks"));
        assert_eq!(
            label_of(LIST_SEEN, "page:recent.markAllSeen"),
            Some("Mark All as Read")
        );
        assert_eq!(
            label_of(ISSUE, "page:issue.markSeen"),
            Some("Mark as Read or Unread")
        );
        assert_eq!(
            label_of(ISSUE, "page:issue.togglePin"),
            Some("Add or Remove Bookmark")
        );
    }

    #[test]
    fn reset_cache_opens_settings_at_the_cache_section_without_a_key() {
        let bus = registered();
        assert_eq!(RESET_CACHE.label, "Reset Cache…");
        assert_eq!(RESET_CACHE.command, "app:settings:cache");
        assert_eq!(RESET_CACHE.accelerator, None);
        assert!(bus.knows(RESET_CACHE.command));
        let info = bus.info(RESET_CACHE.command).unwrap();
        assert_eq!(info.title, "Settings Page");
        assert!(info.hidden);
    }

    #[test]
    fn no_two_menu_items_share_an_accelerator() {
        let mut seen: Vec<&str> = Vec::new();
        for accelerator in all_items().iter().filter_map(|item| item.accelerator) {
            assert!(!seen.contains(&accelerator), "{accelerator}");
            seen.push(accelerator);
        }
    }
}
