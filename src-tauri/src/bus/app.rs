//! The bus inside the app: the app is its host, menus spawn commands, the page runs them with
//! `run_command`.

use tauri::{AppHandle, Manager};

use super::{CommandBus, Host};
use crate::error::{AppError, AppResult};

/// The app's commands, registered once at startup.
pub struct Commands(pub CommandBus<AppHandle>);

impl std::ops::Deref for Commands {
    type Target = CommandBus<AppHandle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Host for AppHandle {
    fn report_failure(&self, id: &str, error: &AppError) {
        log::error!("{id} failed: {error}");
    }
}

/// Runs a command without waiting for it (menu items).
pub fn spawn(app: &AppHandle, id: &str) {
    let app = app.clone();
    let id = id.to_string();
    tauri::async_runtime::spawn(async move {
        let commands = app.state::<Commands>();
        commands.run(app.clone(), &id).await;
    });
}

/// The page runs commands by id; a failure comes back as the error.
#[tauri::command]
pub async fn run_command(app: AppHandle, id: String) -> AppResult<()> {
    let work = app.state::<Commands>().start(app.clone(), &id)?;
    work.await
}
