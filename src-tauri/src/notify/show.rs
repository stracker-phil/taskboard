//! Hands notices to macOS. A notice's button opens its issue (`issue:open:<KEY>`) or the inbox (`inbox:show`).

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::{Action, ActionType, NotificationExt};

use super::Notice;
use crate::bus;

const OPEN_ISSUE: &str = "taskboard-open-issue";
const SHOW_INBOX: &str = "taskboard-show-inbox";

/// The buttons, once per launch; a button's id is the command it runs.
pub fn install(app: &AppHandle) {
    let notification = app.notification();
    let kinds = vec![
        ActionType::builder(OPEN_ISSUE)
            .actions(vec![Action::builder("open", "Open").build()])
            .build(),
        ActionType::builder(SHOW_INBOX)
            .actions(vec![
                Action::builder("inbox:show", "Show Notifications").build(),
            ])
            .build(),
    ];
    if let Err(e) = notification.register_action_types(kinds) {
        log::warn!("Notification buttons won't show: {e}");
    }
    let handle = app.clone();
    let installed = notification.on_action(move |performed| {
        let command = match performed.action_id() {
            "open" => performed
                .notification()
                .and_then(|n| n.extra().get("key"))
                .and_then(|key| key.as_str())
                .map(|key| format!("issue:open:{key}")),
            other => Some(other.to_string()),
        };
        if let Some(command) = command
            && handle.state::<bus::Commands>().knows(&command)
        {
            bus::spawn(&handle, &command);
        }
    });
    if let Err(e) = installed {
        log::warn!("Notification buttons won't work: {e}");
    }
}

pub fn show(app: &AppHandle, notice: &Notice) {
    let mut builder = app
        .notification()
        .builder()
        .title(&notice.title)
        .body(&notice.body);
    builder = match &notice.key {
        Some(key) => builder.action_type_id(OPEN_ISSUE).extra("key", key.clone()),
        None => builder.action_type_id(SHOW_INBOX),
    };
    if let Err(e) = builder.show() {
        log::warn!("Couldn't show a notification ({}): {e}", notice.title);
    }
}
