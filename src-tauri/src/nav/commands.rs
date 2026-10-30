//! The Tauri and bus commands of the main window's place.

use tauri::{AppHandle, Emitter, Manager, State};

use super::Nav;
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
