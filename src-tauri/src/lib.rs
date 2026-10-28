//! Taskboard's library: the modules and `run`, which reads the command line, hands off to a running
//! Taskboard or starts the app.

mod app;
pub mod bus;
pub mod cli;
mod commands;
pub mod connection;
pub mod db;
#[cfg(debug_assertions)]
mod dev;
pub mod error;
pub mod events;
pub mod filter;
mod handlers;
mod instance;
pub mod ipc;
pub mod jira;
pub mod launch;
pub mod lists;
pub mod lock;
pub mod menu;
pub mod nav;
pub mod net;
mod order;
mod remote;
pub mod secrets;
pub mod state;
pub mod status_style;
pub mod store;
pub mod sync;
#[cfg(any(test, debug_assertions, feature = "test-util"))]
pub mod test_support;
pub mod views;
pub mod window;

use std::path::PathBuf;

use crate::cli::{CliOutcome, CliRequest};
use crate::ipc::IpcMessage;
use crate::state::AppState;
use crate::store::json_file::create_private_dir;

/// The bundle identifier and config folder name; the debug build's own keeps its data apart (ADR 0011).
const IDENTIFIER: &str = if cfg!(debug_assertions) {
    "com.philippstracker.taskboard.debug"
} else {
    "com.philippstracker.taskboard"
};
/// Debug builds go by their own name in the menu bar and the Dock.
pub const APP_NAME: &str = if cfg!(debug_assertions) {
    "Taskboard (Debug)"
} else {
    "Taskboard"
};
const SOCKET_NAME: &str = "taskboard.sock";
/// Debug builds claim their own scheme (`tauri.debug.conf.json`), so links reach the installed app.
pub const LINK_SCHEME: &str = if cfg!(debug_assertions) {
    "taskboard-debug"
} else {
    "taskboard"
};

/// Same directory Tauri resolves as `app_config_dir`, but available before the app starts.
fn config_dir() -> PathBuf {
    let Some(home) = std::env::home_dir() else {
        fail("Cannot locate the home directory for Taskboard's settings");
    };
    home.join("Library/Application Support").join(IDENTIFIER)
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(2);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_dir = config_dir();
    if let Err(e) = create_private_dir(&config_dir) {
        eprintln!("Cannot create {}: {e}", config_dir.display());
    }
    let socket = config_dir.join(SOCKET_NAME);

    let (message, log_level, hidden) = match cli::parse(std::env::args_os()) {
        CliOutcome::Exit { message, code } => {
            if code == 0 {
                print!("{message}");
            } else {
                eprint!("{message}");
            }
            std::process::exit(code);
        }
        CliOutcome::Run { hidden, log_level } => {
            (IpcMessage::Activate { hidden }, log_level, hidden)
        }
        CliOutcome::Ask(CliRequest { message, json }) => {
            match instance::ask(&socket, &message, json) {
                Some(code) => std::process::exit(code),
                None => fail("Taskboard isn't running"),
            }
        }
        CliOutcome::Remote(CliRequest { message, json }) => {
            if let Some(code) = instance::ask(&socket, &message, json) {
                std::process::exit(code);
            }
            (message, None, false)
        }
    };

    let _instance_lock = instance::forward_or_lock(&config_dir, &socket, &message);
    let state = AppState::open(&config_dir)
        .unwrap_or_else(|e| fail(&format!("Taskboard can't open its data: {e}")));
    app::start(
        state,
        socket,
        message,
        log_level.unwrap_or_default(),
        hidden,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configs() -> (serde_json::Value, serde_json::Value) {
        let release = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let debug = serde_json::from_str(include_str!("../tauri.debug.conf.json")).unwrap();
        (release, debug)
    }

    fn this_build() -> serde_json::Value {
        let (release, debug) = configs();
        if cfg!(debug_assertions) {
            debug
        } else {
            release
        }
    }

    #[test]
    fn identifier_matches_the_tauri_config() {
        assert_eq!(this_build()["identifier"], serde_json::json!(IDENTIFIER));
    }

    #[test]
    fn app_name_matches_the_product_name_in_the_tauri_config() {
        assert_eq!(this_build()["productName"], serde_json::json!(APP_NAME));
    }

    #[test]
    fn link_scheme_matches_the_deep_link_scheme_in_the_tauri_config() {
        assert_eq!(
            this_build()["plugins"]["deep-link"]["desktop"]["schemes"],
            serde_json::json!([LINK_SCHEME])
        );
    }

    #[test]
    fn the_debug_build_differs_from_the_release_build_in_name_identifier_and_scheme() {
        let (release, debug) = configs();
        assert_eq!(release["productName"], "Taskboard");
        assert_eq!(debug["productName"], "Taskboard (Debug)");
        assert_eq!(
            debug["identifier"],
            serde_json::json!(format!("{}.debug", release["identifier"].as_str().unwrap()))
        );
        let schemes = |config: &serde_json::Value| {
            config["plugins"]["deep-link"]["desktop"]["schemes"].clone()
        };
        assert_eq!(schemes(&release), serde_json::json!(["taskboard"]));
        assert_eq!(schemes(&debug), serde_json::json!(["taskboard-debug"]));
    }
}
