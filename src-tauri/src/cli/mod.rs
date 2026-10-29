//! The `taskboard` command line. Without a subcommand it starts the app (or activates the running one);
//! a subcommand asks the running app over the socket and prints its answer (`reply`). A new subcommand adds a
//! `Command` variant, its `IpcMessage`, its arm in `remote::handle` and, when it prints something, in
//! `reply::render`.

mod listing;
pub mod reply;
pub mod rows;

use std::ffi::OsString;

use clap::{Parser, Subcommand};

use crate::ipc::IpcMessage;
use crate::views::Layout;

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

const EXAMPLES: &str = "Examples:
  taskboard                          start Taskboard or bring it to the front
  taskboard open nw-12               open an issue in the full Issue view
  taskboard spaces                   the tracked Spaces with their keys
  taskboard filters NW               a Space's filter lists with their ids
  taskboard go NW Board              open a Space at one of its views
  taskboard go \"NW/Open bugs\"        open a filter list
  taskboard go recent                open Recently changed
  taskboard me on                    show only my issues in the current view
  taskboard layout table             show the current view as a table
  taskboard search \"login page\" --json
  taskboard sync --full              fetch every issue again
  taskboard status --json | jq .place";

#[derive(Parser, Debug)]
#[command(
    name = "taskboard",
    version,
    about = "A local-first Jira Cloud client for macOS",
    disable_help_subcommand = true,
    after_help = EXAMPLES
)]
struct Cli {
    /// Start without showing the window; a running instance stays as it is
    #[arg(long)]
    hidden: bool,
    /// Log level for this run
    #[arg(long, value_enum)]
    log_level: Option<LogLevel>,
    /// Print replies as JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Whether the app runs, its version, its window, where it is and the sync
    Status,
    /// Show the window without bringing the app to the front; starts the app when it isn't running
    Show,
    /// Show the window and bring the app to the front; starts the app when it isn't running
    Focus,
    /// Hide the window; the app keeps running
    Hide,
    /// Quit the running app
    Quit,
    /// Open an issue by its key (e.g. NW-12) in the full Issue view; starts the app when it isn't running
    Open {
        #[arg(value_parser = issue_key)]
        key: String,
    },
    /// List the tracked Spaces with their keys and ids
    Spaces,
    /// List Everything, Home's lists and the Spaces' filter lists with their ids
    Filters {
        /// Only this Space's lists (its key or name)
        space: Option<String>,
    },
    /// List a level's views with their ids (default: the open level)
    Views {
        /// everything, recent, pinned, a Space's key or name, a filter list's name (KEY/name) or an id
        level: Option<String>,
    },
    /// Open a Space, a filter list or a Home list, at a view or its last one; starts the app when it isn't running
    Go {
        /// everything, recent, pinned, a Space's key or name, a filter list's name (KEY/name) or an id
        level: String,
        /// A view's name or id (default: the one it showed last)
        view: Option<String>,
    },
    /// Show only my issues in the current view: on, off, or flip it without an argument
    Me {
        #[arg(value_parser = switch)]
        on: Option<bool>,
    },
    /// Show the current view as a list, a board or a table
    Layout {
        #[arg(value_enum)]
        layout: LayoutArg,
    },
    /// Find cached issues by key or text, as the command palette does
    Search {
        /// Words to find
        #[arg(required = true, num_args = 1..)]
        text: Vec<String>,
    },
    /// Ask the running app to sync with Jira now; returns at once
    Sync {
        /// Fetch every issue of the sync scope again
        #[arg(long)]
        full: bool,
    },
    /// Empty the local copy of Jira and fetch it again; Spaces, filter lists, views and pins stay
    ResetCache {
        /// Confirm it
        #[arg(long, required = true)]
        yes: bool,
    },
    /// Debug builds only: drive the fake Jira
    #[cfg(debug_assertions)]
    #[command(subcommand)]
    Dev(DevCommand),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum LayoutArg {
    List,
    Board,
    Table,
}

impl From<LayoutArg> for Layout {
    fn from(layout: LayoutArg) -> Self {
        match layout {
            LayoutArg::List => Layout::List,
            LayoutArg::Board => Layout::Board,
            LayoutArg::Table => Layout::Table,
        }
    }
}

#[cfg(debug_assertions)]
#[derive(Subcommand, Debug)]
enum DevCommand {
    /// Change the demo site (a comment, a status, a priority, a new and a deleted issue)
    Drift,
    /// Replace the demo site with generated issues and sync them (for scrolling and timing checks)
    SeedSynthetic {
        /// How many issues
        count: usize,
    },
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
    /// A subcommand the running app answers, or the app starts with it (`show`, `focus`, `open`, `go`).
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
    let Some(command) = cli.command else {
        return CliOutcome::Run {
            hidden: cli.hidden,
            log_level: cli.log_level,
        };
    };
    let starts_app = matches!(
        command,
        Command::Show | Command::Focus | Command::Open { .. } | Command::Go { .. }
    );
    let request = CliRequest {
        message: message(command),
        json: cli.json,
    };
    if starts_app {
        CliOutcome::Remote(request)
    } else {
        CliOutcome::Ask(request)
    }
}

fn message(command: Command) -> IpcMessage {
    match command {
        Command::Status => IpcMessage::Status,
        Command::Show => IpcMessage::Show { focus: false },
        Command::Focus => IpcMessage::Activate { hidden: false },
        Command::Hide => IpcMessage::Hide,
        Command::Quit => IpcMessage::Quit,
        Command::Open { key } => IpcMessage::Open { key },
        Command::Spaces => IpcMessage::Spaces,
        Command::Filters { space } => IpcMessage::Filters { space },
        Command::Views { level } => IpcMessage::Views { level },
        Command::Go { level, view } => IpcMessage::Go { level, view },
        Command::Me { on } => IpcMessage::Me { on },
        Command::Layout { layout } => IpcMessage::Layout {
            layout: layout.into(),
        },
        Command::Search { text } => IpcMessage::Search {
            text: text.join(" "),
        },
        Command::Sync { full } => IpcMessage::Sync { full },
        Command::ResetCache { .. } => IpcMessage::ResetCache,
        #[cfg(debug_assertions)]
        Command::Dev(DevCommand::Drift) => IpcMessage::DevDrift,
        #[cfg(debug_assertions)]
        Command::Dev(DevCommand::SeedSynthetic { count }) => IpcMessage::DevSeedSynthetic { count },
    }
}

/// `on` or `off`, any case.
fn switch(text: &str) -> Result<bool, String> {
    match text.to_ascii_lowercase().as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("“{text}” is neither on nor off")),
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
mod tests;
