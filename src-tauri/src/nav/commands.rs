//! The Tauri and bus commands of the main window's place.

use tauri::{AppHandle, Emitter, Manager, State};

use super::{Direction, Nav};
use crate::bus::{Command, CommandBus};
use crate::error::AppResult;
use crate::events::NAV_CHANGED_EVENT;
use crate::state::AppState;

#[tauri::command]
pub async fn nav_current(state: State<'_, AppState>) -> AppResult<Nav> {
    state.nav.current(&state.db).await
}

/// `view_id: null` opens the level's last or default view.
#[tauri::command]
pub async fn nav_go(
    state: State<'_, AppState>,
    list_id: i64,
    view_id: Option<i64>,
) -> AppResult<Nav> {
    state.nav.go(&state.db, list_id, view_id).await
}

/// A new tab at the level (at `view_id`, else its last or default view; the active tab's place without a level),
/// switched to with `activate`.
#[tauri::command]
pub async fn nav_tab_new(
    state: State<'_, AppState>,
    list_id: Option<i64>,
    view_id: Option<i64>,
    activate: bool,
) -> AppResult<Nav> {
    state
        .nav
        .new_tab(&state.db, list_id, view_id, activate)
        .await
}

#[tauri::command]
pub async fn nav_tab_activate(state: State<'_, AppState>, id: u32) -> AppResult<Nav> {
    state.nav.activate_tab(&state.db, id).await
}

#[tauri::command]
pub async fn nav_tab_close(state: State<'_, AppState>, id: u32) -> AppResult<Nav> {
    state.nav.close_tab(&state.db, id).await
}

/// Moves the tab before `before`, or to the end.
#[tauri::command]
pub async fn nav_tab_move(
    state: State<'_, AppState>,
    id: u32,
    before: Option<u32>,
) -> AppResult<Nav> {
    state.nav.move_tab(&state.db, id, before).await
}

/// The active tab's Issue view shows `issue`, or none.
#[tauri::command]
pub async fn nav_set_issue(state: State<'_, AppState>, issue: Option<String>) -> AppResult<Nav> {
    state.nav.set_issue(&state.db, issue).await
}

/// Back in the active tab's history; `Nav::issue` is the issue to show there.
#[tauri::command]
pub async fn nav_back(state: State<'_, AppState>) -> AppResult<Nav> {
    state.nav.travel(&state.db, Direction::Back).await
}

#[tauri::command]
pub async fn nav_forward(state: State<'_, AppState>) -> AppResult<Nav> {
    state.nav.travel(&state.db, Direction::Forward).await
}

#[tauri::command]
pub async fn nav_set_expanded(state: State<'_, AppState>, expanded: Vec<i64>) -> AppResult<Nav> {
    state.nav.set_expanded(&state.db, expanded).await
}

pub fn register(bus: &mut CommandBus<AppHandle>) {
    bus.add(
        Command::new("nav:previousSpace", "Previous Space")
            .help("Open the Space above")
            .run(|app| step(app, -1)),
    );
    bus.add(
        Command::new("nav:nextSpace", "Next Space")
            .help("Open the Space below")
            .run(|app| step(app, 1)),
    );
    bus.add(
        Command::new("nav:previousList", "Previous List")
            .help("Open the list above, in the same Space")
            .run(|app| step_list(app, -1)),
    );
    bus.add(
        Command::new("nav:nextList", "Next List")
            .help("Open the list below, in the same Space")
            .run(|app| step_list(app, 1)),
    );
    bus.add(
        Command::new("nav:newTab", "New Tab")
            .help("Open the current place in a new tab")
            .run(new_tab),
    );
    bus.add(
        Command::new("nav:nextTab", "Next Tab")
            .help("Show the tab to the right")
            .run(|app| tab_step(app, 1)),
    );
    bus.add(
        Command::new("nav:previousTab", "Previous Tab")
            .help("Show the tab to the left")
            .run(|app| tab_step(app, -1)),
    );
    bus.add(
        Command::new("nav:reopenTab", "Reopen Closed Tab")
            .help("Open the last closed tab again")
            .run(reopen_tab),
    );
}

/// ⌘W: in the main window with more than one tab, closes the active tab; else hides the window in front.
pub fn close_tab_or_window(app: &AppHandle) {
    let state = app.state::<AppState>();
    if crate::window::focused_label(app).as_deref() != Some(crate::window::MAIN)
        || state.nav.tab_count() < 2
    {
        crate::window::close_focused(app);
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        match state.nav.close_tab(&state.db, state.nav.active_tab()).await {
            Ok(nav) => announce(&app, &nav),
            Err(e) => log::warn!("Couldn't close the tab: {e}"),
        }
    });
}

async fn tab_step(app: AppHandle, step: i64) -> AppResult<()> {
    let state = app.state::<AppState>();
    let nav = state.nav.step_tab(&state.db, step).await?;
    announce(&app, &nav);
    Ok(())
}

async fn new_tab(app: AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    let nav = state.nav.new_tab(&state.db, None, None, true).await?;
    announce(&app, &nav);
    Ok(())
}

async fn reopen_tab(app: AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    let nav = state.nav.reopen_tab(&state.db).await?;
    announce(&app, &nav);
    Ok(())
}

async fn step(app: AppHandle, step: i64) -> AppResult<()> {
    let state = app.state::<AppState>();
    let nav = state.nav.step_space(&state.db, step).await?;
    announce(&app, &nav);
    Ok(())
}

async fn step_list(app: AppHandle, step: i64) -> AppResult<()> {
    let state = app.state::<AppState>();
    let nav = state.nav.step_list(&state.db, step).await?;
    announce(&app, &nav);
    Ok(())
}

fn announce(app: &AppHandle, nav: &Nav) {
    if let Err(e) = app.emit(NAV_CHANGED_EVENT, nav) {
        log::warn!("Couldn't announce the new place: {e}");
    }
}
