//! The app menu's items as the command palette lists them.

use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use super::{MENUS, PALETTE};
use crate::bus::{self, CommandStatus};

#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct MenuCommand {
    /// The menu's title, the palette's section.
    pub menu: String,
    pub label: String,
    pub command: String,
    /// In Tauri's notation (`CmdOrCtrl+Shift+R`).
    pub accelerator: Option<String>,
}

/// Every enabled item of the menus, in menu order; Command Palette… itself is left out.
pub fn listed(commands: &dyn CommandStatus) -> Vec<MenuCommand> {
    MENUS
        .iter()
        .flat_map(|(menu, groups)| {
            groups
                .iter()
                .flat_map(|items| items.iter())
                .map(move |item| (*menu, item))
        })
        .filter(|(_, item)| item.command != PALETTE.command && commands.is_enabled(item.command))
        .map(|(menu, item)| MenuCommand {
            menu: menu.to_string(),
            label: item.label.to_string(),
            command: item.command.to_string(),
            accelerator: item.accelerator.map(str::to_string),
        })
        .collect()
}

#[tauri::command]
pub fn menu_commands(app: AppHandle) -> Vec<MenuCommand> {
    let registry = app.state::<bus::Commands>();
    listed(&registry.available(&app))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::CommandBus;

    /// Every command is known and enabled, except the ones listed.
    struct Except(&'static [&'static str]);

    impl CommandStatus for Except {
        fn knows(&self, id: &str) -> bool {
            !self.0.contains(&id)
        }

        fn is_enabled(&self, id: &str) -> bool {
            self.knows(id)
        }
    }

    fn commands_of(listed: &[MenuCommand]) -> Vec<&str> {
        listed.iter().map(|c| c.command.as_str()).collect()
    }

    fn in_menu<'a>(listed: &'a [MenuCommand], menu: &str) -> Vec<&'a str> {
        listed
            .iter()
            .filter(|c| c.menu == menu)
            .map(|c| c.command.as_str())
            .collect()
    }

    #[test]
    fn the_first_item_is_settings_in_the_app_menu() {
        let items = listed(&Except(&[]));

        assert_eq!(
            items[0],
            MenuCommand {
                menu: crate::APP_NAME.to_string(),
                label: "Settings…".into(),
                command: "app:settings".into(),
                accelerator: Some("CmdOrCtrl+,".into()),
            }
        );
    }

    #[test]
    fn items_come_in_menu_order_under_their_menu_titles() {
        let items = listed(&Except(&[]));

        let mut titles: Vec<&str> = items.iter().map(|c| c.menu.as_str()).collect();
        titles.dedup();
        assert_eq!(
            titles,
            [
                crate::APP_NAME,
                "File",
                "Go",
                "List",
                "Issue",
                "View",
                "Help"
            ]
        );
    }

    #[test]
    fn the_file_menu_lists_its_groups_in_order_with_labels_and_keys() {
        let items = listed(&Except(&[]));

        let file: Vec<(&str, &str, Option<&str>)> = items
            .iter()
            .filter(|c| c.menu == "File")
            .map(|c| {
                (
                    c.label.as_str(),
                    c.command.as_str(),
                    c.accelerator.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            file,
            [
                ("New Filter List", "page:list.new", Some("CmdOrCtrl+N")),
                ("New View", "page:view.new", Some("CmdOrCtrl+T")),
                ("Refresh", "sync:refresh", Some("CmdOrCtrl+R")),
                ("Full Sync", "sync:full", Some("CmdOrCtrl+Shift+R")),
                ("Reset Cache…", "app:settings:cache", None),
                ("Close Window", "window:close", Some("CmdOrCtrl+W")),
            ]
        );
    }

    #[test]
    fn the_palette_item_itself_is_left_out() {
        let items = listed(&Except(&[]));

        assert!(!commands_of(&items).contains(&"page:palette.open"));
        assert_eq!(
            in_menu(&items, "Go"),
            [
                "page:go.everything",
                "page:go.recent",
                "page:go.pinned",
                "nav:previousSpace",
                "nav:nextSpace",
                "page:view.previous",
                "page:view.next",
            ]
        );
    }

    #[test]
    fn items_whose_command_is_not_enabled_are_left_out() {
        let items = listed(&Except(&["app:reload", "sync:full"]));

        let commands = commands_of(&items);
        assert!(!commands.contains(&"app:reload"));
        assert!(!commands.contains(&"sync:full"));
        assert!(commands.contains(&"sync:refresh"));
        assert_eq!(in_menu(&items, "View"), ["view:toggleSidebar"]);
    }

    #[test]
    fn page_items_keep_their_page_ids() {
        let items = listed(&Except(&[]));

        assert_eq!(
            in_menu(&items, "Issue"),
            [
                "page:issue.open",
                "page:issue.togglePeek",
                "page:issue.openEpic",
                "page:issue.openInJira",
                "page:issue.copyKey",
                "page:issue.copyUrl",
                "page:issue.togglePin",
                "page:issue.markSeen",
            ]
        );
    }

    #[test]
    fn the_registered_bus_lists_reload_only_in_debug_builds() {
        let mut bus = CommandBus::<AppHandle>::default();
        bus::register(&mut bus);
        crate::nav::commands::register(&mut bus);

        struct Known<'a>(&'a CommandBus<AppHandle>);
        impl CommandStatus for Known<'_> {
            fn knows(&self, id: &str) -> bool {
                self.0.knows(id)
            }
            fn is_enabled(&self, id: &str) -> bool {
                self.0.knows(id)
            }
        }

        let items = listed(&Known(&bus));

        assert_eq!(
            commands_of(&items).contains(&"app:reload"),
            cfg!(debug_assertions)
        );
        assert_eq!(items[0].command, "app:settings");
    }

    #[test]
    fn a_menu_command_serializes_with_camel_case_fields() {
        let item = MenuCommand {
            menu: "File".into(),
            label: "New View".into(),
            command: "page:view.new".into(),
            accelerator: None,
        };

        assert_eq!(
            serde_json::to_value(&item).unwrap(),
            serde_json::json!({
                "menu": "File",
                "label": "New View",
                "command": "page:view.new",
                "accelerator": null,
            })
        );
    }
}
