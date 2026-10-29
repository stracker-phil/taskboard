//! Settings → Status: what the Jira connection, the sync and this Mac report now (`groups` builds the texts).

pub mod groups;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use tauri::{AppHandle, Manager, State};

use groups::{Diagnostics, JiraFacts, LocalFacts, SyncFacts, jira_group, local_group, sync_group};

use crate::connection;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::sync::{SyncEngine, SyncState, overview};

/// A stored UTC time as the owner reads it, e.g. "29 Oct, 09:14"; unreadable text stays as it is.
fn local(utc: &str) -> String {
    DateTime::parse_from_rfc3339(utc).map_or_else(
        |_| utc.to_string(),
        |at| at.with_timezone(&Local).format("%-d %b, %H:%M").to_string(),
    )
}

/// A path with the home folder as `~`, as Finder and the terminal show it.
fn tilde(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => Path::new("~").join(rest).display().to_string(),
        None => path.display().to_string(),
    }
}

/// Settings → Status's groups, read now.
#[tauri::command]
pub async fn diagnostics_get(
    app: AppHandle,
    state: State<'_, AppState>,
    sync: State<'_, SyncEngine>,
) -> AppResult<Diagnostics> {
    let view = connection::view(&state.settings, &state.jira, Local::now().date_naive());
    let jira = JiraFacts {
        site_url: view.connection.site_url,
        email: view.connection.email,
        display_name: view.connection.display_name,
        has_token: view.has_token,
        expiry: view.expiry,
    };
    let sync_state = sync.state();
    let last_ok = match &sync_state {
        SyncState::Idle { last_ok } => last_ok.as_deref().map(local),
        _ => None,
    };
    let counts = overview(&state.db, sync_state.clone()).await?;
    let sync_facts = SyncFacts {
        state: sync_state,
        last_ok,
        last_full: counts.last_full_at.as_deref().map(local),
        tracked: counts.projects.len(),
        cached: counts.projects.iter().map(|p| p.cached).sum(),
    };
    let paths = app.path();
    let home = paths.home_dir().ok();
    let shown =
        |dir: Result<PathBuf, String>| dir.map_or_else(|e| e, |dir| tilde(&dir, home.as_deref()));
    let local_facts = LocalFacts {
        data_dir: shown(paths.app_config_dir().map_err(|e| e.to_string())),
        log_dir: shown(paths.app_log_dir().map_err(|e| e.to_string())),
        cli_path: shown(std::env::current_exe().map_err(|e| e.to_string())),
        version: app.package_info().version.to_string(),
    };
    Ok(Diagnostics {
        groups: vec![
            jira_group(&jira),
            sync_group(&sync_facts),
            local_group(&local_facts),
        ],
    })
}

/// Shows the folder with the settings, the cache and the socket in Finder.
pub fn open_data_folder(app: AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Io(e.to_string()))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/Users/sam.example";

    #[test]
    fn a_path_under_home_shows_with_a_tilde() {
        let home = Path::new(HOME);
        for (path, expected) in [
            (
                "/Users/sam.example/Library/Logs/Taskboard",
                "~/Library/Logs/Taskboard",
            ),
            ("/Users/sam.example/notes.txt", "~/notes.txt"),
        ] {
            assert_eq!(tilde(Path::new(path), Some(home)), expected, "{path}");
        }
    }

    #[test]
    fn the_home_folder_itself_shows_as_a_bare_tilde() {
        assert_eq!(tilde(Path::new(HOME), Some(Path::new(HOME))), "~");
    }

    #[test]
    fn a_path_outside_home_stays_unchanged() {
        let home = Path::new(HOME);
        for path in [
            "/Applications/Taskboard.app/Contents/MacOS/taskboard",
            "/Users/sam.example2/Documents",
            "/Users/other.example/Documents",
        ] {
            assert_eq!(tilde(Path::new(path), Some(home)), path, "{path}");
        }
    }

    #[test]
    fn a_path_stays_unchanged_without_a_home_folder() {
        let path = "/Users/sam.example/Documents";
        assert_eq!(tilde(Path::new(path), None), path);
    }

    #[test]
    fn unreadable_time_text_is_returned_as_is() {
        for text in ["", "yesterday", "2026-10-29 09:14", "2026-13-40T99:00:00Z"] {
            assert_eq!(local(text), text, "{text:?}");
        }
    }

    #[test]
    fn a_stored_utc_time_shows_in_the_local_zone_as_day_month_and_time() {
        let utc = "2026-10-29T09:14:00Z";
        let instant = DateTime::parse_from_rfc3339(utc)
            .unwrap()
            .with_timezone(&Local);
        let expected = format!(
            "{} {}, {}",
            instant.format("%-d"),
            instant.format("%b"),
            instant.format("%H:%M")
        );
        assert_eq!(local(utc), expected);

        let shown = local(utc);
        let (date, time) = shown
            .split_once(", ")
            .expect("date and time are comma separated");
        let (day, month) = date
            .split_once(' ')
            .expect("day and month are space separated");
        assert!((1..=31).contains(&day.parse::<u32>().unwrap()), "{shown}");
        assert!(
            [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
            ]
            .contains(&month),
            "{shown}"
        );
        assert_eq!(time.len(), 5, "{shown}");
        assert_eq!(time.as_bytes()[2], b':', "{shown}");
    }

    #[test]
    fn an_offset_time_shows_the_same_as_the_equal_utc_instant() {
        assert_eq!(
            local("2026-10-29T11:14:00+02:00"),
            local("2026-10-29T09:14:00Z")
        );
    }
}
