//! The commands a notification's button runs.

use tauri::AppHandle;

use crate::bus::{Command, CommandBus};
use crate::window;

pub fn register(bus: &mut CommandBus<AppHandle>) {
    bus.add_family(
        Command::new("issue:open:", "Open Issue")
            .help("Show the window with the issue (by key) in the full Issue view")
            .hidden()
            .family("<key>")
            .run(|app, key| async move {
                window::show_main(&app);
                crate::remote::open_issue(&app, key).await
            }),
    );
    bus.add(
        Command::new("unread:show", "Show Unread")
            .help("Show the window at Unread")
            .hidden()
            .run(|app| async move {
                window::show_main(&app);
                window::run_page_command(&app, "page:go.recent");
                Ok(())
            }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registered() -> CommandBus<AppHandle> {
        let mut bus = CommandBus::default();
        register(&mut bus);
        bus
    }

    #[test]
    fn opening_an_issue_by_key_is_a_hidden_family() {
        let bus = registered();

        for id in ["issue:open:NW-1042", "issue:open:LUM-309"] {
            assert!(bus.knows(id), "{id}");
            let info = bus.info(id).unwrap();
            assert_eq!(info.title, "Open Issue", "{id}");
            assert!(info.hidden, "{id}");
        }
        assert_eq!(bus.family("issue:open:").unwrap().arg, "<key>");
    }

    #[test]
    fn showing_unread_is_a_hidden_command() {
        let bus = registered();

        assert!(bus.knows("unread:show"));
        let info = bus.info("unread:show").unwrap();
        assert_eq!(info.title, "Show Unread");
        assert!(info.hidden);
    }
}
