//! What the command prints for the app's reply, one fact per line so scripts can grep it. `--json`
//! prints the reply itself instead.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::listing;
use super::rows::{IssueRow, LevelRow, PlaceReport, SpaceRow, ViewsReport};
use crate::ipc::IpcMessage;
use crate::sync::{SyncOverview, SyncState};

/// The reply to `status`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    pub version: String,
    pub window_visible: bool,
    pub sync: SyncOverview,
    /// Where the main window is; none while no level exists.
    #[serde(default)]
    pub place: Option<PlaceReport>,
}

/// The text for `result`, the reply to `message`; `None` when the subcommand prints nothing.
pub fn render(message: &IpcMessage, result: &Value) -> Result<Option<String>, String> {
    Ok(match message {
        IpcMessage::Status => Some(status_text(&read::<StatusReport>(result)?)),
        IpcMessage::Spaces => Some(listing::spaces(&read::<Vec<SpaceRow>>(result)?)),
        IpcMessage::Filters { .. } => Some(listing::levels(&read::<Vec<LevelRow>>(result)?)),
        IpcMessage::Views { .. } => Some(listing::views(&read::<ViewsReport>(result)?)),
        IpcMessage::Go { .. } => read::<Option<PlaceReport>>(result)?.map(|p| listing::place(&p)),
        IpcMessage::Search { .. } => Some(listing::issues(&read::<Vec<IssueRow>>(result)?)),
        IpcMessage::ResetCache => Some(read::<String>(result)?),
        #[cfg(debug_assertions)]
        IpcMessage::DevDrift | IpcMessage::DevSeedSynthetic { .. } => Some(read::<String>(result)?),
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
    let mut lines = vec![
        format!("Taskboard {}", report.version),
        format!("window: {window}"),
        format!(
            "at: {}",
            report
                .place
                .as_ref()
                .map_or("nothing open".into(), listing::place)
        ),
        format!("sync: {}", sync_text(&report.sync.state)),
        format!(
            "last full sync: {}",
            report.sync.last_full_at.as_deref().unwrap_or("never")
        ),
    ];
    lines.extend(
        report
            .sync
            .projects
            .iter()
            .map(|p| format!("{} {}: {} cached, {} open", p.key, p.name, p.cached, p.open)),
    );
    lines.join("\n")
}

fn sync_text(state: &SyncState) -> String {
    match state {
        SyncState::Idle { .. } => "idle".into(),
        SyncState::Running { .. } => "running".into(),
        SyncState::Error { message, .. } => format!("error: {message}"),
        SyncState::Offline { since } => format!("offline since {since}"),
        SyncState::Paused { message } => format!("paused: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::sync::{Job, ProjectCounts};
    use serde_json::json;

    const SYNCED_AT: &str = "2026-10-28T10:00:00.000Z";

    fn idle() -> SyncState {
        SyncState::Idle { last_ok: None }
    }

    fn overview(
        state: SyncState,
        last_full_at: Option<&str>,
        projects: Vec<ProjectCounts>,
    ) -> SyncOverview {
        SyncOverview {
            state,
            last_full_at: last_full_at.map(String::from),
            projects,
        }
    }

    fn counts(key: &str, name: &str, cached: u32, open: u32) -> ProjectCounts {
        ProjectCounts {
            id: "10001".into(),
            key: key.into(),
            name: name.into(),
            cached,
            open,
            last_full_at: None,
        }
    }

    fn report(window_visible: bool, sync: SyncOverview) -> Value {
        serde_json::to_value(status(window_visible, sync, None)).unwrap()
    }

    fn status(
        window_visible: bool,
        sync: SyncOverview,
        place: Option<PlaceReport>,
    ) -> StatusReport {
        StatusReport {
            version: "1.2.3".into(),
            window_visible,
            sync,
            place,
        }
    }

    fn status_text_of(reply: &Value) -> String {
        render(&IpcMessage::Status, reply).unwrap().unwrap()
    }

    #[test]
    fn status_prints_the_version_and_a_visible_window() {
        let reply = report(true, overview(idle(), None, vec![]));
        assert_eq!(
            status_text_of(&reply),
            "Taskboard 1.2.3\nwindow: visible\nat: nothing open\nsync: idle\nlast full sync: never"
        );
    }

    #[test]
    fn status_prints_a_hidden_window() {
        let reply = report(false, overview(idle(), None, vec![]));
        assert_eq!(
            status_text_of(&reply),
            "Taskboard 1.2.3\nwindow: hidden\nat: nothing open\nsync: idle\nlast full sync: never"
        );
    }

    #[test]
    fn status_prints_where_the_window_is() {
        let place = PlaceReport {
            list_id: 2,
            level: "NW Website relaunch".into(),
            view_id: 5,
            view: "Board".into(),
        };
        let reply = serde_json::to_value(status(true, overview(idle(), None, vec![]), Some(place)))
            .unwrap();
        assert_eq!(
            status_text_of(&reply),
            "Taskboard 1.2.3\nwindow: visible\nat: NW Website relaunch › Board\nsync: idle\nlast full sync: never"
        );
    }

    #[test]
    fn a_status_reply_without_a_place_still_reads() {
        let reply = json!({
            "version": "1.2.3",
            "windowVisible": true,
            "sync": { "state": { "state": "idle", "lastOk": null }, "lastFullAt": null, "projects": [] }
        });
        assert_eq!(
            status_text_of(&reply),
            "Taskboard 1.2.3\nwindow: visible\nat: nothing open\nsync: idle\nlast full sync: never"
        );
    }

    #[test]
    fn status_prints_the_last_full_sync_and_one_line_per_project() {
        let reply = report(
            true,
            overview(
                SyncState::Idle {
                    last_ok: Some(SYNCED_AT.into()),
                },
                Some(SYNCED_AT),
                vec![
                    counts("NW", "Website relaunch", 33, 21),
                    counts("NWS", "Northwind support", 8, 6),
                ],
            ),
        );
        assert_eq!(
            status_text_of(&reply),
            "Taskboard 1.2.3\n\
             window: visible\n\
             at: nothing open\n\
             sync: idle\n\
             last full sync: 2026-10-28T10:00:00.000Z\n\
             NW Website relaunch: 33 cached, 21 open\n\
             NWS Northwind support: 8 cached, 6 open"
        );
    }

    #[test]
    fn status_prints_every_sync_state_on_one_line() {
        for (state, line) in [
            (idle(), "sync: idle"),
            (SyncState::Running { job: Job::Full }, "sync: running"),
            (
                SyncState::Error {
                    kind: ErrorKind::Internal,
                    message: "Jira answered 500".into(),
                },
                "sync: error: Jira answered 500",
            ),
            (
                SyncState::Offline {
                    since: SYNCED_AT.into(),
                },
                "sync: offline since 2026-10-28T10:00:00.000Z",
            ),
            (
                SyncState::Paused {
                    message: "Jira refused the credentials".into(),
                },
                "sync: paused: Jira refused the credentials",
            ),
        ] {
            let reply = report(true, overview(state.clone(), None, vec![]));
            assert_eq!(
                status_text_of(&reply),
                format!(
                    "Taskboard 1.2.3\nwindow: visible\nat: nothing open\n{line}\nlast full sync: never"
                ),
                "{state:?}"
            );
        }
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
            IpcMessage::Sync { full: false },
            IpcMessage::Sync { full: true },
            IpcMessage::Me { on: None },
            IpcMessage::Layout {
                layout: crate::views::Layout::Board,
            },
        ] {
            assert_eq!(render(&message, &json!(null)), Ok(None), "{message:?}");
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn dev_drift_prints_the_apps_text_as_it_is() {
        let reply = json!("Added a comment to NW-1042");
        assert_eq!(
            render(&IpcMessage::DevDrift, &reply),
            Ok(Some("Added a comment to NW-1042".to_string()))
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_dev_drift_reply_that_is_not_text_is_an_error() {
        let error = render(&IpcMessage::DevDrift, &json!({ "done": true })).unwrap_err();
        assert!(
            error.starts_with("The running Taskboard sent an unexpected reply"),
            "{error}"
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn dev_seed_synthetic_prints_the_apps_text_as_it_is() {
        let message = IpcMessage::DevSeedSynthetic { count: 5000 };
        assert_eq!(
            render(&message, &json!("Synced 5000 issues")),
            Ok(Some("Synced 5000 issues".to_string()))
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_dev_seed_synthetic_reply_that_is_not_text_is_an_error() {
        let message = IpcMessage::DevSeedSynthetic { count: 5000 };
        let error = render(&message, &json!({ "done": true })).unwrap_err();
        assert!(
            error.starts_with("The running Taskboard sent an unexpected reply"),
            "{error}"
        );
    }

    #[test]
    fn a_status_reply_of_the_wrong_shape_is_an_error() {
        for reply in [
            json!(null),
            json!({ "version": "1.2.3" }),
            json!({ "windowVisible": true }),
            json!({ "version": "1.2.3", "windowVisible": true }),
            json!({
                "version": "1.2.3",
                "windowVisible": true,
                "sync": { "state": { "state": "idle", "lastOk": null } }
            }),
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
        let sync = overview(
            idle(),
            Some(SYNCED_AT),
            vec![ProjectCounts {
                last_full_at: Some(SYNCED_AT.into()),
                ..counts("NW", "Website relaunch", 33, 21)
            }],
        );
        let value = report(false, sync.clone());
        assert_eq!(
            value,
            json!({
                "version": "1.2.3",
                "windowVisible": false,
                "place": null,
                "sync": {
                    "state": { "state": "idle", "lastOk": null },
                    "lastFullAt": SYNCED_AT,
                    "projects": [{
                        "id": "10001",
                        "key": "NW",
                        "name": "Website relaunch",
                        "cached": 33,
                        "open": 21,
                        "lastFullAt": SYNCED_AT
                    }]
                }
            })
        );
        assert_eq!(
            serde_json::from_value::<StatusReport>(value).unwrap(),
            status(false, sync, None)
        );
    }
}
