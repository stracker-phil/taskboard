//! The app's windows: the main window here, Settings in `settings`. Closing either hides it; the app keeps
//! running until it quits (ADR 0016).

pub mod settings;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::events::{PAGE_COMMAND_EVENT, TOGGLE_SIDEBAR_EVENT, WINDOW_OPENED_EVENT};

/// The main window's label.
pub const MAIN: &str = "main";

/// Unminimizes and focuses the window; skipped with `TASKBOARD_BACKGROUND=1` in debug builds so
/// scripts/dev never steal focus.
fn bring_to_front(window: &WebviewWindow) {
    #[cfg(debug_assertions)]
    if crate::dev::background() {
        return;
    }
    // Failing only means the window is gone; there is nothing left to bring forward.
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Shows the hidden or minimized main window and brings it to the front.
pub fn show_main(app: &AppHandle) {
    show_main_in_background(app);
    if let Some(window) = app.get_webview_window(MAIN) {
        bring_to_front(&window);
    }
}

/// Shows the main window without bringing the app to the front; emits `WINDOW_OPENED_EVENT` when it
/// was hidden.
pub fn show_main_in_background(app: &AppHandle) {
    let opening = !main_visible(app);
    if let Some(window) = app.get_webview_window(MAIN)
        && let Err(e) = window.show()
    {
        log::warn!("Couldn't show the Taskboard window: {e}");
    }
    if opening && let Err(e) = app.emit(WINDOW_OPENED_EVENT, ()) {
        log::warn!("Couldn't tell the page the window opened: {e}");
    }
}

fn is_visible(app: &AppHandle, label: &str) -> bool {
    app.get_webview_window(label)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

pub fn main_visible(app: &AppHandle) -> bool {
    is_visible(app, MAIN)
}

fn hide(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label)
        && let Err(e) = window.hide()
    {
        log::warn!("Couldn't hide the {label} window: {e}");
    }
}

pub fn hide_main(app: &AppHandle) {
    hide(app, MAIN);
}

/// Hides the main window or Settings on close and returns true, so the close doesn't proceed.
pub fn close_requested(app: &AppHandle, label: &str) -> bool {
    let ours = label == MAIN || label == settings::LABEL;
    if ours {
        hide(app, label);
    }
    ours
}

fn focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
}

/// ⌘W: hides the window in front.
pub fn close_focused(app: &AppHandle) {
    if let Some(window) = focused_window(app) {
        hide(app, window.label());
    }
}

/// Shows the main window and shows or hides its sidebar.
/// Shows the main window and asks its page to run one of its own commands (`page:<id>` from the menus).
pub fn run_page_command(app: &AppHandle, id: &str) {
    show_main(app);
    if let Err(e) = app.emit_to(MAIN, PAGE_COMMAND_EVENT, id) {
        log::warn!("Couldn't ask the window to run {id}: {e}");
    }
}

pub fn toggle_sidebar(app: &AppHandle) {
    show_main(app);
    if let Err(e) = app.emit_to(MAIN, TOGGLE_SIDEBAR_EVENT, ()) {
        log::warn!("Couldn't toggle the sidebar: {e}");
    }
}

/// Reloads the page of the window in front (debug builds).
#[cfg(debug_assertions)]
pub fn reload_focused(app: &AppHandle) {
    if let Some(window) = focused_window(app)
        && let Err(e) = window.eval("location.reload()")
    {
        log::warn!("Couldn't reload the page: {e}");
    }
}
