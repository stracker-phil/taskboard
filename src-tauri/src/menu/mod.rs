//! The app menu (Taskboard, Edit, View, Window, Help). Every custom item's id is the command it runs; the
//! standard items (About, Services, Hide, the Edit items, Minimize) are macOS's own.

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
pub const VIEW: &[Item] = &[item(
    "Toggle Sidebar",
    "view:toggleSidebar",
    "CmdOrCtrl+Backslash",
)];
pub const RELOAD: Item = item("Reload", "app:reload", "CmdOrCtrl+Shift+R");
pub const CLOSE: Item = item("Close Window", "window:close", "CmdOrCtrl+W");
pub const HELP: &[Item] = &[plain("Open Log Folder", "app:logs")];

/// `None` for a command no feature registered.
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
    let view = with_items(app, &commands, SubmenuBuilder::new(app, "View"), VIEW)?;
    let view = with_items(app, &commands, view, &[RELOAD])?.build()?;
    let window = SubmenuBuilder::new(app, "Window").minimize().maximize();
    let window = with_items(app, &commands, window.separator(), &[CLOSE])?
        .separator()
        .bring_all_to_front()
        .build()?;
    let help = with_items(app, &commands, SubmenuBuilder::new(app, "Help"), HELP)?.build()?;
    Menu::with_items(app, &[&taskboard, &edit, &view, &window, &help])
}

/// A click on any menu item runs the command of its id.
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
        bus
    }

    #[test]
    fn every_menu_item_runs_a_registered_command() {
        let bus = registered();
        let mut items: Vec<&Item> = vec![&SETTINGS, &QUIT, &CLOSE];
        items.extend(VIEW);
        items.extend(HELP);
        if cfg!(debug_assertions) {
            items.push(&RELOAD);
        }
        for item in items {
            assert!(bus.knows(item.command), "{}", item.command);
        }
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
}
