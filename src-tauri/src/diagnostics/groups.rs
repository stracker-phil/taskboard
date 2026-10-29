//! Builds the Settings → Status groups from facts the app gathers (`super`). Pure: no I/O.

use serde::Serialize;
use ts_rs::TS;

use crate::connection::TokenExpiry;
use crate::sync::{Job, SyncState};

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub groups: Vec<DiagnosticGroup>,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticGroup {
    pub title: String,
    pub rows: Vec<DiagnosticRow>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticTone {
    Ok,
    Warn,
    Error,
    Info,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRow {
    pub label: String,
    pub value: String,
    pub tone: DiagnosticTone,
    /// A bus command the page offers as a button.
    pub command: Option<String>,
    pub command_label: Option<String>,
}

fn row(label: &str, value: impl Into<String>, tone: DiagnosticTone) -> DiagnosticRow {
    DiagnosticRow {
        label: label.into(),
        value: value.into(),
        tone,
        command: None,
        command_label: None,
    }
}

impl DiagnosticRow {
    fn offering(mut self, command: &str, label: &str) -> Self {
        self.command = Some(command.into());
        self.command_label = Some(label.into());
        self
    }
}

fn group(title: &str, rows: Vec<DiagnosticRow>) -> DiagnosticGroup {
    DiagnosticGroup {
        title: title.into(),
        rows,
    }
}

const CONNECTION_PAGE: &str = "app:settings:connection";

pub struct JiraFacts {
    pub site_url: String,
    pub email: String,
    /// From the last successful Test Connection.
    pub display_name: Option<String>,
    pub has_token: bool,
    pub expiry: TokenExpiry,
}

pub fn jira_group(facts: &JiraFacts) -> DiagnosticGroup {
    use DiagnosticTone::*;
    if facts.site_url.is_empty() {
        return group(
            "Jira",
            vec![row("Site", "Not set up", Warn).offering(CONNECTION_PAGE, "Set Up…")],
        );
    }
    let account = match &facts.display_name {
        Some(name) => row("Account", format!("{name} ({})", facts.email), Ok),
        None => row("Account", format!("{} · not tested yet", facts.email), Warn)
            .offering(CONNECTION_PAGE, "Test…"),
    };
    let token = match (facts.has_token, facts.expiry) {
        (false, _) => {
            row("API token", "Not in the Keychain", Error).offering(CONNECTION_PAGE, "Add…")
        }
        (true, TokenExpiry::Expired) => {
            row("API token", "Expired", Error).offering(CONNECTION_PAGE, "Replace…")
        }
        (true, TokenExpiry::Soon { days: 0 }) => {
            row("API token", "Expires today", Warn).offering(CONNECTION_PAGE, "Replace…")
        }
        (true, TokenExpiry::Soon { days }) => row(
            "API token",
            format!(
                "Expires in {days} {}",
                if days == 1 { "day" } else { "days" }
            ),
            Warn,
        )
        .offering(CONNECTION_PAGE, "Replace…"),
        (true, TokenExpiry::Fine) => row("API token", "In the Keychain", Ok),
    };
    group(
        "Jira",
        vec![
            row("Site", facts.site_url.trim_end_matches('/'), Info),
            account,
            token,
        ],
    )
}

pub struct SyncFacts {
    pub state: SyncState,
    /// Local times, ready to show.
    pub last_ok: Option<String>,
    pub last_full: Option<String>,
    pub tracked: usize,
    pub cached: u32,
}

pub fn sync_group(facts: &SyncFacts) -> DiagnosticGroup {
    use DiagnosticTone::*;
    let state = match &facts.state {
        SyncState::Idle { .. } => match &facts.last_ok {
            Some(at) => row("State", format!("Up to date · {at}"), Ok),
            None => row("State", "Not synced yet", Info).offering("sync:full", "Sync Now"),
        },
        SyncState::Running { job } => row("State", running(*job), Info),
        SyncState::Error { message, .. } => {
            row("State", message.clone(), Error).offering("sync:refresh", "Retry")
        }
        SyncState::Offline { .. } => {
            row("State", "Offline; retrying", Warn).offering("sync:refresh", "Retry")
        }
        SyncState::Paused { message } => {
            row("State", message.clone(), Error).offering(CONNECTION_PAGE, "Fix…")
        }
    };
    let projects = match facts.tracked {
        0 => row("Projects", "None tracked", Warn).offering("app:settings:projects", "Choose…"),
        1 => row("Projects", "1 tracked", Info),
        n => row("Projects", format!("{n} tracked"), Info),
    };
    let last_full = row(
        "Last full sync",
        facts.last_full.clone().unwrap_or_else(|| "Never".into()),
        Info,
    )
    .offering("sync:full", "Full Sync");
    let cached = row("Cached issues", facts.cached.to_string(), Info);
    group("Sync", vec![state, projects, last_full, cached])
}

fn running(job: Job) -> &'static str {
    match job {
        Job::Full => "Running a full sync",
        Job::Delta => "Fetching changes",
        Job::Reconcile => "Checking for deleted issues",
    }
}

pub struct LocalFacts {
    pub data_dir: String,
    pub log_dir: String,
    /// The binary `taskboard` CLI calls run.
    pub cli_path: String,
    pub version: String,
}

pub fn local_group(facts: &LocalFacts) -> DiagnosticGroup {
    use DiagnosticTone::*;
    group(
        "This Mac",
        vec![
            row("Version", facts.version.clone(), Info),
            row("Data folder", facts.data_dir.clone(), Info).offering("app:data", "Show"),
            row("Log folder", facts.log_dir.clone(), Info).offering("app:logs", "Show"),
            row("Command-line tool", facts.cli_path.clone(), Info),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    fn jira(overrides: impl FnOnce(&mut JiraFacts)) -> JiraFacts {
        let mut facts = JiraFacts {
            site_url: "https://example.atlassian.net".into(),
            email: "sam@example.com".into(),
            display_name: Some("Sam Example".into()),
            has_token: true,
            expiry: TokenExpiry::Fine,
        };
        overrides(&mut facts);
        facts
    }

    fn sync(overrides: impl FnOnce(&mut SyncFacts)) -> SyncFacts {
        let mut facts = SyncFacts {
            state: SyncState::Idle {
                last_ok: Some("2026-03-02T09:00:00Z".into()),
            },
            last_ok: Some("09:00".into()),
            last_full: Some("Mon 09:00".into()),
            tracked: 2,
            cached: 120,
        };
        overrides(&mut facts);
        facts
    }

    fn expected(
        label: &str,
        value: &str,
        tone: DiagnosticTone,
        offer: Option<(&str, &str)>,
    ) -> DiagnosticRow {
        DiagnosticRow {
            label: label.into(),
            value: value.into(),
            tone,
            command: offer.map(|(command, _)| command.into()),
            command_label: offer.map(|(_, label)| label.into()),
        }
    }

    fn state_row(state: SyncState, last_ok: Option<&str>) -> DiagnosticRow {
        let facts = sync(|f| {
            f.state = state;
            f.last_ok = last_ok.map(Into::into);
        });
        sync_group(&facts).rows.remove(0)
    }

    #[test]
    fn an_empty_site_gives_a_single_not_set_up_row_offering_setup() {
        let group = jira_group(&jira(|f| f.site_url.clear()));

        assert_eq!(group.title, "Jira");
        assert_eq!(
            group.rows,
            [expected(
                "Site",
                "Not set up",
                DiagnosticTone::Warn,
                Some((CONNECTION_PAGE, "Set Up…"))
            )]
        );
    }

    #[test]
    fn the_site_value_drops_a_trailing_slash() {
        for (site, shown) in [
            (
                "https://example.atlassian.net/",
                "https://example.atlassian.net",
            ),
            (
                "https://example.atlassian.net",
                "https://example.atlassian.net",
            ),
        ] {
            let group = jira_group(&jira(|f| f.site_url = site.into()));
            assert_eq!(
                group.rows[0],
                expected("Site", shown, DiagnosticTone::Info, None),
                "{site}"
            );
        }
    }

    #[test]
    fn a_tested_account_shows_name_and_email_as_ok() {
        let group = jira_group(&jira(|_| {}));

        assert_eq!(
            group.rows[1],
            expected(
                "Account",
                "Sam Example (sam@example.com)",
                DiagnosticTone::Ok,
                None
            )
        );
    }

    #[test]
    fn an_untested_account_warns_and_offers_a_test() {
        let group = jira_group(&jira(|f| f.display_name = None));

        assert_eq!(
            group.rows[1],
            expected(
                "Account",
                "sam@example.com · not tested yet",
                DiagnosticTone::Warn,
                Some((CONNECTION_PAGE, "Test…"))
            )
        );
    }

    #[test]
    fn the_token_row_reflects_presence_and_expiry() {
        use DiagnosticTone::*;
        let replace = Some((CONNECTION_PAGE, "Replace…"));
        let cases = [
            (
                false,
                TokenExpiry::Fine,
                "Not in the Keychain",
                Error,
                Some((CONNECTION_PAGE, "Add…")),
            ),
            (
                false,
                TokenExpiry::Expired,
                "Not in the Keychain",
                Error,
                Some((CONNECTION_PAGE, "Add…")),
            ),
            (true, TokenExpiry::Expired, "Expired", Error, replace),
            (
                true,
                TokenExpiry::Soon { days: 0 },
                "Expires today",
                Warn,
                replace,
            ),
            (
                true,
                TokenExpiry::Soon { days: 1 },
                "Expires in 1 day",
                Warn,
                replace,
            ),
            (
                true,
                TokenExpiry::Soon { days: 5 },
                "Expires in 5 days",
                Warn,
                replace,
            ),
            (true, TokenExpiry::Fine, "In the Keychain", Ok, None),
        ];
        for (has_token, expiry, value, tone, offer) in cases {
            let group = jira_group(&jira(|f| {
                f.has_token = has_token;
                f.expiry = expiry;
            }));
            assert_eq!(
                group.rows[2],
                expected("API token", value, tone, offer),
                "{has_token} {expiry:?}"
            );
        }
    }

    #[test]
    fn an_idle_sync_with_a_last_success_is_up_to_date() {
        let row = state_row(SyncState::Idle { last_ok: None }, Some("09:00"));

        assert_eq!(
            row,
            expected("State", "Up to date · 09:00", DiagnosticTone::Ok, None)
        );
    }

    #[test]
    fn an_idle_sync_without_a_success_offers_to_sync_now() {
        let row = state_row(SyncState::Idle { last_ok: None }, None);

        assert_eq!(
            row,
            expected(
                "State",
                "Not synced yet",
                DiagnosticTone::Info,
                Some(("sync:full", "Sync Now"))
            )
        );
    }

    #[test]
    fn a_running_job_is_described_by_its_kind() {
        for (job, value) in [
            (Job::Full, "Running a full sync"),
            (Job::Delta, "Fetching changes"),
            (Job::Reconcile, "Checking for deleted issues"),
        ] {
            let row = state_row(SyncState::Running { job }, None);
            assert_eq!(
                row,
                expected("State", value, DiagnosticTone::Info, None),
                "{value}"
            );
        }
    }

    #[test]
    fn a_failed_sync_shows_its_message_and_offers_a_retry() {
        let row = state_row(
            SyncState::Error {
                kind: ErrorKind::Internal,
                message: "Jira answered with an unexpected page".into(),
            },
            None,
        );

        assert_eq!(
            row,
            expected(
                "State",
                "Jira answered with an unexpected page",
                DiagnosticTone::Error,
                Some(("sync:refresh", "Retry"))
            )
        );
    }

    #[test]
    fn an_offline_sync_warns_and_offers_a_retry() {
        let row = state_row(
            SyncState::Offline {
                since: "2026-03-02T09:00:00Z".into(),
            },
            None,
        );

        assert_eq!(
            row,
            expected(
                "State",
                "Offline; retrying",
                DiagnosticTone::Warn,
                Some(("sync:refresh", "Retry"))
            )
        );
    }

    #[test]
    fn a_paused_sync_is_an_error_pointing_at_the_connection_page() {
        let row = state_row(
            SyncState::Paused {
                message: "Jira refused the API token".into(),
            },
            None,
        );

        assert_eq!(
            row,
            expected(
                "State",
                "Jira refused the API token",
                DiagnosticTone::Error,
                Some((CONNECTION_PAGE, "Fix…"))
            )
        );
    }

    #[test]
    fn the_projects_row_counts_tracked_projects() {
        use DiagnosticTone::*;
        let cases = [
            (
                0,
                "None tracked",
                Warn,
                Some(("app:settings:projects", "Choose…")),
            ),
            (1, "1 tracked", Info, None),
            (3, "3 tracked", Info, None),
        ];
        for (tracked, value, tone, offer) in cases {
            let group = sync_group(&sync(|f| f.tracked = tracked));
            assert_eq!(
                group.rows[1],
                expected("Projects", value, tone, offer),
                "{tracked}"
            );
        }
    }

    #[test]
    fn the_last_full_sync_reads_never_when_unknown_and_always_offers_a_full_sync() {
        for (last_full, value) in [(None, "Never"), (Some("Mon 09:00"), "Mon 09:00")] {
            let group = sync_group(&sync(|f| f.last_full = last_full.map(Into::into)));
            assert_eq!(
                group.rows[2],
                expected(
                    "Last full sync",
                    value,
                    DiagnosticTone::Info,
                    Some(("sync:full", "Full Sync"))
                ),
                "{value}"
            );
        }
    }

    #[test]
    fn the_cached_issue_count_is_shown_as_text() {
        let group = sync_group(&sync(|f| f.cached = 1234));

        assert_eq!(group.title, "Sync");
        assert_eq!(group.rows.len(), 4);
        assert_eq!(
            group.rows[3],
            expected("Cached issues", "1234", DiagnosticTone::Info, None)
        );
    }

    #[test]
    fn the_local_group_lists_four_rows_with_show_commands_for_the_folders() {
        let group = local_group(&LocalFacts {
            data_dir: "/Users/example/Library/Application Support/Taskboard".into(),
            log_dir: "/Users/example/Library/Logs/Taskboard".into(),
            cli_path: "/Applications/Taskboard.app/Contents/MacOS/taskboard".into(),
            version: "1.2.3".into(),
        });

        assert_eq!(group.title, "This Mac");
        assert_eq!(
            group.rows,
            [
                expected("Version", "1.2.3", DiagnosticTone::Info, None),
                expected(
                    "Data folder",
                    "/Users/example/Library/Application Support/Taskboard",
                    DiagnosticTone::Info,
                    Some(("app:data", "Show"))
                ),
                expected(
                    "Log folder",
                    "/Users/example/Library/Logs/Taskboard",
                    DiagnosticTone::Info,
                    Some(("app:logs", "Show"))
                ),
                expected(
                    "Command-line tool",
                    "/Applications/Taskboard.app/Contents/MacOS/taskboard",
                    DiagnosticTone::Info,
                    None
                ),
            ]
        );
    }

    #[test]
    fn diagnostics_serialize_with_camel_case_fields_and_lowercase_tones() {
        let diagnostics = Diagnostics {
            groups: vec![group(
                "Jira",
                vec![
                    row("Site", "Not set up", DiagnosticTone::Warn)
                        .offering("sync:full", "Sync Now"),
                ],
            )],
        };

        assert_eq!(
            serde_json::to_value(&diagnostics).unwrap(),
            serde_json::json!({
                "groups": [{
                    "title": "Jira",
                    "rows": [{
                        "label": "Site",
                        "value": "Not set up",
                        "tone": "warn",
                        "command": "sync:full",
                        "commandLabel": "Sync Now"
                    }]
                }]
            })
        );
    }
}
