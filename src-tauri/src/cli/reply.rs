//! What the command prints for the app's reply, one fact per line so scripts can grep it. `--json`
//! prints the reply itself instead.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ipc::IpcMessage;

/// The reply to `status`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    pub version: String,
    pub window_visible: bool,
}

/// The text for `result`, the reply to `message`; `None` when the subcommand prints nothing.
pub fn render(message: &IpcMessage, result: &Value) -> Result<Option<String>, String> {
    Ok(match message {
        IpcMessage::Status => Some(status_text(&read::<StatusReport>(result)?)),
        _ => None,
    })
}

fn read<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    T::deserialize(value)
        .map_err(|e| format!("The running Taskboard sent an unexpected reply: {e}"))
}

fn status_text(report: &StatusReport) -> String {
    let window = if report.window_visible {
        "visible"
    } else {
        "hidden"
    };
    format!("Taskboard {}\nwindow: {window}", report.version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn status_prints_the_version_and_a_visible_window() {
        let reply = json!({ "version": "1.2.3", "windowVisible": true });
        assert_eq!(
            render(&IpcMessage::Status, &reply),
            Ok(Some("Taskboard 1.2.3\nwindow: visible".to_string()))
        );
    }

    #[test]
    fn status_prints_a_hidden_window() {
        let reply = json!({ "version": "1.2.3", "windowVisible": false });
        assert_eq!(
            render(&IpcMessage::Status, &reply),
            Ok(Some("Taskboard 1.2.3\nwindow: hidden".to_string()))
        );
    }

    #[test]
    fn other_messages_print_nothing() {
        for message in [
            IpcMessage::Hide,
            IpcMessage::Quit,
            IpcMessage::Show { focus: false },
            IpcMessage::Open {
                key: "NW-12".into(),
            },
        ] {
            assert_eq!(render(&message, &json!(null)), Ok(None), "{message:?}");
        }
    }

    #[test]
    fn a_status_reply_of_the_wrong_shape_is_an_error() {
        for reply in [
            json!(null),
            json!({ "version": "1.2.3" }),
            json!({ "windowVisible": true }),
        ] {
            let error = render(&IpcMessage::Status, &reply).unwrap_err();
            assert!(
                error.starts_with("The running Taskboard sent an unexpected reply"),
                "{reply}: {error}"
            );
        }
    }

    #[test]
    fn a_status_report_uses_camel_case_keys_and_round_trips() {
        let report = StatusReport {
            version: "1.2.3".into(),
            window_visible: false,
        };
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value, json!({ "version": "1.2.3", "windowVisible": false }));
        assert_eq!(
            serde_json::from_value::<StatusReport>(value).unwrap(),
            report
        );
    }
}
