//! The core Tauri commands: settings, key bindings and UI state, and the small OS helpers (links, the log
//! folder) the page calls.

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::events::{KEYBINDINGS_CHANGED_EVENT, SETTINGS_CHANGED_EVENT};
use crate::launch::LaunchEvent;
use crate::state::AppState;
use crate::store::keybindings::Keybindings;
use crate::store::settings::Settings;
use crate::store::ui_state::UiState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    let (_, saved) = state.settings.update(settings).await?;
    if let Err(e) = app.emit(SETTINGS_CHANGED_EVENT, &saved) {
        log::warn!("Couldn't announce the saved settings: {e}");
    }
    log::info!("Settings updated");
    Ok(saved)
}

#[tauri::command]
pub fn get_keybindings(state: State<'_, AppState>) -> Keybindings {
    state.keybindings.get()
}

#[tauri::command]
pub async fn set_keybindings(
    app: AppHandle,
    state: State<'_, AppState>,
    bindings: Keybindings,
) -> AppResult<Keybindings> {
    let saved = state.keybindings.update(bindings).await?;
    if let Err(e) = app.emit(KEYBINDINGS_CHANGED_EVENT, &saved) {
        log::warn!("Couldn't announce the saved shortcuts: {e}");
    }
    Ok(saved)
}

#[tauri::command]
pub fn get_ui_state(state: State<'_, AppState>) -> UiState {
    state.ui_state.get()
}

#[tauri::command]
pub async fn set_ui_state(state: State<'_, AppState>, ui: UiState) -> AppResult<UiState> {
    state.ui_state.set(ui).await
}

#[tauri::command]
pub fn open_log_folder(app: AppHandle) -> AppResult<()> {
    let dir = app
        .path()
        .app_log_dir()
        .map_err(|e| AppError::Io(e.to_string()))?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::Io(e.to_string()))
}

/// https only, and without credentials, which the browser would send on.
pub fn is_web_url(url: &str) -> bool {
    url::Url::parse(url)
        .is_ok_and(|u| u.scheme() == "https" && u.username().is_empty() && u.password().is_none())
}

/// Opens a link in the default browser; the webview itself never navigates away.
#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> AppResult<()> {
    if !is_web_url(&url) {
        return Err(AppError::InvalidInput(format!("Not a web link: {url}")));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Io(e.to_string()))
}

/// The page listens: it takes what waited for it.
#[tauri::command]
pub fn take_launch_events(state: State<'_, AppState>) -> Vec<LaunchEvent> {
    state.launches.take_events()
}

#[tauri::command]
pub fn log_frontend_error(message: String) {
    log::error!(target: "frontend", "{}", sanitize_log_text(&message));
}

const MAX_LOGGED_BYTES: usize = 4096;

/// Bounds page-supplied text and escapes line breaks, so it cannot flood the log or forge log lines.
pub fn sanitize_log_text(text: &str) -> String {
    text[..text.floor_char_boundary(MAX_LOGGED_BYTES)]
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_url_rules_allow_https_only_without_credentials() {
        for (url, allowed) in [
            ("https://example.com/path", true),
            ("HTTPS://example.com", true),
            ("http://example.com", false),
            ("http://localhost:8080", false),
            ("file:///tmp/notes.txt", false),
            ("javascript:alert(1)", false),
            ("mailto:a@example.com", false),
            ("not a url", false),
            ("", false),
            ("https://user:pw@example.com", false),
            ("https://user@example.com", false),
        ] {
            assert_eq!(is_web_url(url), allowed, "{url}");
        }
    }

    #[test]
    fn logged_text_is_cut_on_a_char_boundary() {
        let text = format!("{}é", "a".repeat(MAX_LOGGED_BYTES - 1));
        assert_eq!(sanitize_log_text(&text), "a".repeat(MAX_LOGGED_BYTES - 1));
        assert_eq!(
            sanitize_log_text(&"x".repeat(10_000)),
            "x".repeat(MAX_LOGGED_BYTES)
        );
    }

    #[test]
    fn short_text_is_kept_whole() {
        assert_eq!(sanitize_log_text("Something broke"), "Something broke");
    }

    #[test]
    fn logged_text_escapes_line_breaks() {
        assert_eq!(sanitize_log_text("a\nb\rc"), "a\\nb\\rc");
    }
}
