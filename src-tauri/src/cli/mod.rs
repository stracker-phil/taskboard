//! The `taskboard` command line. Without a subcommand it starts the app (or activates the running one);
//! a subcommand asks the running app over the socket and prints its answer (`reply`).

pub mod reply;

use std::ffi::OsString;

use clap::{Parser, Subcommand};

use crate::ipc::IpcMessage;

/// The `--log-level` of the CLI; without it the app logs at `Info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "taskboard",
    version,
    about = "A local-first Jira Cloud client for macOS",
    disable_help_subcommand = true
)]
struct Cli {
    /// Start without showing the window; a running instance stays as it is
    #[arg(long)]
    hidden: bool,
    /// Log level for this run
    #[arg(long, value_enum)]
    log_level: Option<LogLevel>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Whether the app runs, its version and its window
    Status {
        /// Print the reply as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show the window without bringing the app to the front; starts the app when it isn't running
    Show,
    /// Show the window and bring the app to the front; starts the app when it isn't running
    Focus,
    /// Hide the window; the app keeps running
    Hide,
    /// Open an issue by its key, e.g. NW-12
    Open {
        #[arg(value_parser = issue_key)]
        key: String,
    },
    /// Quit the running app
    Quit,
    /// Ask the running app to sync with Jira now; returns at once
    Sync {
        /// A full sync of the whole scope (the default until delta syncs exist)
        #[arg(long)]
        full: bool,
    },
    /// Debug builds only: drive the fake Jira
    #[cfg(debug_assertions)]
    #[command(subcommand)]
    Dev(DevCommand),
}

#[cfg(debug_assertions)]
#[derive(Subcommand, Debug)]
enum DevCommand {
    /// Change the demo site (a comment, a status, a priority, a new and a deleted issue)
    Drift,
}

/// A message for the running app and how to print its reply.
#[derive(Debug, PartialEq)]
pub struct CliRequest {
    pub message: IpcMessage,
    pub json: bool,
}

#[derive(Debug, PartialEq)]
pub enum CliOutcome {
    /// Start the app, or hand the launch to the running one.
    Run {
        hidden: bool,
        log_level: Option<LogLevel>,
    },
    /// `--help`, `--version` or a usage error: print `message` and exit with `code` (0 or 2).
    Exit { message: String, code: i32 },
    /// A subcommand the running app answers; it never starts the app.
    Ask(CliRequest),
    /// `show` or `focus`: the running app answers it, or the app starts with it.
    Remote(CliRequest),
}

/// Parses the full argv (including the program name).
pub fn parse<I, S>(args: I) -> CliOutcome
where
    I: IntoIterator<Item = S>,
    S: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            return CliOutcome::Exit {
                message: e.render().to_string(),
                code: e.exit_code(),
            };
        }
    };
    let ask = |message, json| CliOutcome::Ask(CliRequest { message, json });
    match cli.command {
        None => CliOutcome::Run {
            hidden: cli.hidden,
            log_level: cli.log_level,
        },
        Some(Command::Status { json }) => ask(IpcMessage::Status, json),
        Some(Command::Show) => CliOutcome::Remote(CliRequest {
            message: IpcMessage::Show { focus: false },
            json: false,
        }),
        Some(Command::Focus) => CliOutcome::Remote(CliRequest {
            message: IpcMessage::Activate { hidden: false },
            json: false,
        }),
        Some(Command::Hide) => ask(IpcMessage::Hide, false),
        Some(Command::Open { key }) => ask(IpcMessage::Open { key }, false),
        Some(Command::Quit) => ask(IpcMessage::Quit, false),
        Some(Command::Sync { full }) => ask(IpcMessage::Sync { full }, false),
        #[cfg(debug_assertions)]
        Some(Command::Dev(DevCommand::Drift)) => ask(IpcMessage::DevDrift, false),
    }
}

/// A Jira issue key (`NW-12`, any case), upper-cased.
fn issue_key(text: &str) -> Result<String, String> {
    let key = text.trim().to_ascii_uppercase();
    let valid = key.split_once('-').is_some_and(|(project, number)| {
        project.starts_with(|c: char| c.is_ascii_uppercase())
            && project
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            && !number.is_empty()
            && !number.starts_with('0')
            && number.chars().all(|c| c.is_ascii_digit())
    });
    if valid {
        Ok(key)
    } else {
        Err(format!("“{text}” is not an issue key such as NW-12"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> CliOutcome {
        parse(std::iter::once("taskboard").chain(args.iter().copied()))
    }

    fn ask(message: IpcMessage, json: bool) -> CliOutcome {
        CliOutcome::Ask(CliRequest { message, json })
    }

    fn exit_code(outcome: CliOutcome) -> i32 {
        match outcome {
            CliOutcome::Exit { code, .. } => code,
            other => panic!("expected an exit, got {other:?}"),
        }
    }

    #[test]
    fn no_arguments_start_the_app_visibly_with_the_default_log_level() {
        assert_eq!(
            parse_args(&[]),
            CliOutcome::Run {
                hidden: false,
                log_level: None
            }
        );
    }

    #[test]
    fn hidden_starts_the_app_without_a_window() {
        assert_eq!(
            parse_args(&["--hidden"]),
            CliOutcome::Run {
                hidden: true,
                log_level: None
            }
        );
    }

    #[test]
    fn every_log_level_is_accepted() {
        for (word, level) in [
            ("error", LogLevel::Error),
            ("warn", LogLevel::Warn),
            ("info", LogLevel::Info),
            ("debug", LogLevel::Debug),
            ("trace", LogLevel::Trace),
        ] {
            assert_eq!(
                parse_args(&["--log-level", word]),
                CliOutcome::Run {
                    hidden: false,
                    log_level: Some(level)
                },
                "{word}"
            );
        }
    }

    #[test]
    fn hidden_and_log_level_combine() {
        assert_eq!(
            parse_args(&["--hidden", "--log-level", "debug"]),
            CliOutcome::Run {
                hidden: true,
                log_level: Some(LogLevel::Debug)
            }
        );
    }

    #[test]
    fn an_unknown_log_level_is_a_usage_error() {
        assert_eq!(exit_code(parse_args(&["--log-level", "loud"])), 2);
    }

    #[test]
    fn log_levels_map_to_log_filters() {
        for (level, filter) in [
            (LogLevel::Error, log::LevelFilter::Error),
            (LogLevel::Warn, log::LevelFilter::Warn),
            (LogLevel::Info, log::LevelFilter::Info),
            (LogLevel::Debug, log::LevelFilter::Debug),
            (LogLevel::Trace, log::LevelFilter::Trace),
        ] {
            assert_eq!(log::LevelFilter::from(level), filter, "{level:?}");
        }
        assert_eq!(LogLevel::default(), LogLevel::Info);
    }

    #[test]
    fn version_prints_the_name_and_version_and_exits_cleanly() {
        match parse_args(&["--version"]) {
            CliOutcome::Exit { message, code } => {
                assert_eq!(code, 0);
                assert_eq!(
                    message.trim_end(),
                    format!("taskboard {}", env!("CARGO_PKG_VERSION"))
                );
            }
            other => panic!("expected an exit, got {other:?}"),
        }
    }

    #[test]
    fn help_exits_cleanly_and_lists_the_subcommands() {
        match parse_args(&["--help"]) {
            CliOutcome::Exit { message, code } => {
                assert_eq!(code, 0);
                for word in [
                    "status", "show", "focus", "hide", "open", "quit", "sync", "--hidden",
                ] {
                    assert!(message.contains(word), "{word}");
                }
            }
            other => panic!("expected an exit, got {other:?}"),
        }
    }

    #[test]
    fn status_asks_the_running_app_in_text_or_json() {
        assert_eq!(parse_args(&["status"]), ask(IpcMessage::Status, false));
        assert_eq!(
            parse_args(&["status", "--json"]),
            ask(IpcMessage::Status, true)
        );
    }

    #[test]
    fn show_does_not_ask_for_focus_and_may_start_the_app() {
        assert_eq!(
            parse_args(&["show"]),
            CliOutcome::Remote(CliRequest {
                message: IpcMessage::Show { focus: false },
                json: false
            })
        );
    }

    #[test]
    fn focus_activates_the_visible_window_and_may_start_the_app() {
        assert_eq!(
            parse_args(&["focus"]),
            CliOutcome::Remote(CliRequest {
                message: IpcMessage::Activate { hidden: false },
                json: false
            })
        );
    }

    #[test]
    fn hide_and_quit_ask_the_running_app() {
        assert_eq!(parse_args(&["hide"]), ask(IpcMessage::Hide, false));
        assert_eq!(parse_args(&["quit"]), ask(IpcMessage::Quit, false));
    }

    #[test]
    fn sync_asks_the_running_app_for_a_sync_and_full_asks_for_the_whole_scope() {
        assert_eq!(
            parse_args(&["sync"]),
            ask(IpcMessage::Sync { full: false }, false)
        );
        assert_eq!(
            parse_args(&["sync", "--full"]),
            ask(IpcMessage::Sync { full: true }, false)
        );
    }

    #[test]
    fn sync_rejects_unknown_flags() {
        assert_eq!(exit_code(parse_args(&["sync", "--fully"])), 2);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn dev_drift_asks_the_running_app_to_change_the_demo_site() {
        assert_eq!(
            parse_args(&["dev", "drift"]),
            ask(IpcMessage::DevDrift, false)
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn dev_without_a_known_subcommand_is_a_usage_error() {
        for args in [&["dev"][..], &["dev", "explode"]] {
            assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
        }
    }

    #[test]
    fn open_upper_cases_the_issue_key() {
        for (input, key) in [
            ("nw-12", "NW-12"),
            ("NW-12", "NW-12"),
            ("Ab_c2-7", "AB_C2-7"),
        ] {
            assert_eq!(
                parse_args(&["open", input]),
                ask(IpcMessage::Open { key: key.into() }, false),
                "{input}"
            );
        }
    }

    #[test]
    fn open_rejects_malformed_issue_keys() {
        for input in [
            "12", "NW-", "NW-012", "-12", "N W-1", "NW-1a", "1W-5", "NW-0", "",
        ] {
            assert_eq!(exit_code(parse_args(&["open", input])), 2, "{input:?}");
        }
    }

    #[test]
    fn open_without_a_key_is_a_usage_error() {
        assert_eq!(exit_code(parse_args(&["open"])), 2);
    }

    #[test]
    fn unknown_subcommands_and_flags_are_usage_errors() {
        for args in [
            &["frobnicate"][..],
            &["--frobnicate"],
            &["status", "--frobnicate"],
        ] {
            assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
        }
    }
}
