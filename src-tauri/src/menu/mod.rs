//! The app menu (Taskboard, File, Edit, Go, List, Issue, View, Window, Help). Every custom item's id is the command
//! it runs; `page:<id>` items run the main window's own commands (the same ids its keys run). The standard
//! items (About, Services, Hide, the Edit items, Minimize) are macOS's own. Plain-letter keys (J, K, P, M) stay
//! off the menu items, so they never take a typed letter from a text field.

pub mod palette;
pub mod place;

use tauri::menu::{
    AboutMetadata, CheckMenuItemBuilder, IsMenuItem, Menu, MenuItemBuilder, Submenu, SubmenuBuilder,
};
use tauri::{AppHandle, Manager, Wry};

use crate::bus::{self, CommandStatus};

use self::place::{Place, VIEW_KEYS};

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
pub const PALETTE: Item = item("Command Palette…", "page:palette.open", "CmdOrCtrl+K");
pub const GO_LEVELS: &[Item] = &[
    PALETTE,
    item("Everything", "page:go.everything", "CmdOrCtrl+Shift+E"),
    item("Unread", "page:go.recent", "CmdOrCtrl+Shift+U"),
    item("Bookmarks", "page:go.pinned", "CmdOrCtrl+Shift+P"),
];
pub const GO_MOVES: &[Item] = &[
    item("Previous Space", "nav:previousSpace", "CmdOrCtrl+Alt+Up"),
    item("Next Space", "nav:nextSpace", "CmdOrCtrl+Alt+Down"),
    item(
        "Previous List",
        "nav:previousList",
        "CmdOrCtrl+Alt+BracketLeft",
    ),
    item("Next List", "nav:nextList", "CmdOrCtrl+Alt+BracketRight"),
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
    item("Save View", "page:view.save", "CmdOrCtrl+S"),
    plain("Revert View", "page:view.revert"),
];
pub const LIST_SEEN: &[Item] = &[item(
    "Mark All as Read",
    "page:recent.markAllSeen",
    "CmdOrCtrl+Shift+A",
)];
pub const ISSUE: &[Item] = &[
    plain("Open", "page:issue.open"),
    plain("Open or Close Preview", "page:issue.togglePeek"),
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

/// A menu's title and its own items, in the groups it splits with separators; macOS's standard items come
/// on top.
pub type MenuDef = (&'static str, &'static [&'static [Item]]);

pub const APP_MENU: MenuDef = (crate::APP_NAME, &[&[SETTINGS], &[QUIT]]);
pub const FILE_MENU: MenuDef = ("File", &[FILE, FILE_SYNC, &[RESET_CACHE], &[CLOSE]]);
pub const GO_MENU: MenuDef = ("Go", &[GO_LEVELS, GO_MOVES]);
/// The layouts come last: changing a view's layout is rare.
pub const LIST_MENU: MenuDef = ("List", &[LIST_TOOLBAR, LIST_SEEN, LIST_LAYOUTS]);
pub const ISSUE_MENU: MenuDef = ("Issue", &[ISSUE]);
pub const VIEW_MENU: MenuDef = ("View", &[VIEW, &[RELOAD]]);
pub const HELP_MENU: MenuDef = ("Help", &[HELP]);
/// Every menu with items of its own, in menu bar order.
pub const MENUS: &[MenuDef] = &[
    APP_MENU, FILE_MENU, GO_MENU, LIST_MENU, ISSUE_MENU, VIEW_MENU, HELP_MENU,
];

/// What every item is built with: the app, which commands exist and run, and the current place.
struct Ctx<'a> {
    app: &'a AppHandle,
    commands: &'a dyn CommandStatus,
    place: &'a Place,
}

type Builder<'a> = SubmenuBuilder<'a, Wry, AppHandle>;

fn build_item(ctx: &Ctx, item: &Item) -> tauri::Result<Option<Box<dyn IsMenuItem<Wry>>>> {
    if !ctx.commands.knows(item.command) {
        return Ok(None);
    }
    let enabled = ctx.commands.is_enabled(item.command);
    let built: Box<dyn IsMenuItem<Wry>> = match ctx.place.checked(item.command) {
        Some(checked) => {
            let mut builder = CheckMenuItemBuilder::with_id(item.command, item.label)
                .checked(checked)
                .enabled(enabled);
            if let Some(accelerator) = item.accelerator {
                builder = builder.accelerator(accelerator);
            }
            Box::new(builder.build(ctx.app)?)
        }
        None => {
            let mut builder = MenuItemBuilder::with_id(item.command, item.label).enabled(enabled);
            if let Some(accelerator) = item.accelerator {
                builder = builder.accelerator(accelerator);
            }
            Box::new(builder.build(ctx.app)?)
        }
    };
    Ok(Some(built))
}

fn with_items<'a>(
    ctx: &Ctx<'a>,
    mut builder: Builder<'a>,
    items: &[Item],
) -> tauri::Result<Builder<'a>> {
    for item in items {
        if let Some(built) = build_item(ctx, item)? {
            builder = builder.item(&*built);
        }
    }
    Ok(builder)
}

/// A menu's groups with a separator between each two; a group without a known command is left out.
fn with_groups<'a>(
    ctx: &Ctx<'a>,
    mut builder: Builder<'a>,
    groups: &[&[Item]],
) -> tauri::Result<Builder<'a>> {
    let shown = groups
        .iter()
        .filter(|items| items.iter().any(|item| ctx.commands.knows(item.command)));
    for (i, items) in shown.enumerate() {
        if i > 0 {
            builder = builder.separator();
        }
        builder = with_items(ctx, builder, items)?;
    }
    Ok(builder)
}

fn submenu<'a>(ctx: &Ctx<'a>, (title, groups): MenuDef) -> tauri::Result<Builder<'a>> {
    with_groups(ctx, SubmenuBuilder::new(ctx.app, title), groups)
}

/// The level's first nine views by name, the shown one checked, each with its ⌘ digit.
fn with_views<'a>(ctx: &Ctx<'a>, mut builder: Builder<'a>) -> tauri::Result<Builder<'a>> {
    if ctx.place.views.is_empty() {
        return Ok(builder);
    }
    builder = builder.separator();
    for (i, (id, name)) in ctx.place.views.iter().take(VIEW_KEYS).enumerate() {
        let item = CheckMenuItemBuilder::with_id(format!("page:view.select.{}", i + 1), name)
            .checked(ctx.place.view_id == Some(*id))
            .accelerator(format!("CmdOrCtrl+{}", i + 1))
            .build(ctx.app)?;
        builder = builder.item(&item);
    }
    Ok(builder)
}

fn app_submenu<'a>(ctx: &Ctx<'a>) -> tauri::Result<Submenu<Wry>> {
    let builder = SubmenuBuilder::new(ctx.app, APP_MENU.0)
        .about(Some(AboutMetadata::default()))
        .separator();
    let builder = with_items(ctx, builder, &[SETTINGS])?
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator();
    with_items(ctx, builder, &[QUIT])?.build()
}

/// The menu bar for `place`; `place::refresh` builds it again when the level's views change.
pub fn build(app: &AppHandle, place: &Place) -> tauri::Result<Menu<Wry>> {
    let registry = app.state::<bus::Commands>();
    let commands = registry.available(app);
    let ctx = Ctx {
        app,
        commands: &commands,
        place,
    };
    let taskboard = app_submenu(&ctx)?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let file = submenu(&ctx, FILE_MENU)?.build()?;
    let go = with_views(&ctx, submenu(&ctx, GO_MENU)?)?.build()?;
    let list = submenu(&ctx, LIST_MENU)?.build()?;
    let issue = submenu(&ctx, ISSUE_MENU)?.build()?;
    // The View menu keeps its items together, above Enter Full Screen.
    let view = with_items(&ctx, SubmenuBuilder::new(app, VIEW_MENU.0), VIEW)?;
    let view = with_items(&ctx, view, &[RELOAD])?
        .separator()
        .fullscreen()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .bring_all_to_front()
        .build()?;
    let help = submenu(&ctx, HELP_MENU)?.build()?;
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
        MENUS
            .iter()
            .flat_map(|(_, groups)| groups.iter().flat_map(|items| items.iter()))
            .collect()
    }

    #[test]
    fn every_item_const_appears_in_the_menus_exactly_once() {
        let mut expected: Vec<&str> = [&SETTINGS, &QUIT, &RESET_CACHE, &CLOSE, &RELOAD]
            .iter()
            .map(|item| item.command)
            .collect();
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
            expected.extend(list.iter().map(|item| item.command));
        }
        let mut found: Vec<&str> = all_items().iter().map(|item| item.command).collect();
        expected.sort();
        found.sort();
        assert_eq!(found, expected);
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
            LIST_TOOLBAR,
            LIST_SEEN,
            LIST_LAYOUTS,
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
                "toolbar.group",
                "toolbar.columns",
                "toolbar.filter",
                "toolbar.me",
                "toolbar.search",
                "view.save",
                "view.revert",
                "recent.markAllSeen",
                "view.layout.list",
                "view.layout.board",
                "view.layout.table",
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
    fn the_space_and_list_moves_run_the_nav_commands() {
        let commands: Vec<(&str, &str, Option<&str>)> = GO_MOVES
            .iter()
            .filter(|item| !item.command.starts_with("page:"))
            .map(|item| (item.label, item.command, item.accelerator))
            .collect();
        assert_eq!(
            commands,
            [
                (
                    "Previous Space",
                    "nav:previousSpace",
                    Some("CmdOrCtrl+Alt+Up")
                ),
                ("Next Space", "nav:nextSpace", Some("CmdOrCtrl+Alt+Down")),
                (
                    "Previous List",
                    "nav:previousList",
                    Some("CmdOrCtrl+Alt+BracketLeft")
                ),
                (
                    "Next List",
                    "nav:nextList",
                    Some("CmdOrCtrl+Alt+BracketRight")
                ),
            ]
        );
    }

    #[test]
    fn the_layouts_come_last_in_the_list_menu() {
        let first_commands: Vec<&str> = LIST_MENU.1.iter().map(|g| g[0].command).collect();
        assert_eq!(
            first_commands,
            [
                LIST_TOOLBAR[0].command,
                LIST_SEEN[0].command,
                LIST_LAYOUTS[0].command
            ]
        );
    }

    #[test]
    fn the_digit_keys_of_the_views_do_not_clash_with_any_item() {
        let digits: Vec<String> = (1..=VIEW_KEYS).map(|n| format!("CmdOrCtrl+{n}")).collect();
        for accelerator in all_items().iter().filter_map(|item| item.accelerator) {
            assert!(!digits.iter().any(|d| d == accelerator), "{accelerator}");
        }
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
    fn save_and_revert_view_close_the_list_toolbar_menu() {
        let tail: Vec<(&str, &str, Option<&str>)> = LIST_TOOLBAR
            .iter()
            .rev()
            .take(2)
            .rev()
            .map(|item| (item.label, item.command, item.accelerator))
            .collect();
        assert_eq!(
            tail,
            [
                ("Save View", "page:view.save", Some("CmdOrCtrl+S")),
                ("Revert View", "page:view.revert", None),
            ]
        );
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
