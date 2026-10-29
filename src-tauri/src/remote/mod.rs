//! Requests from the CLI: `handle` maps each `IpcMessage` to the module that does it.

mod levels;
mod places;

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::cli::reply::StatusReport;
use crate::error::AppError;
use crate::ipc::{HandlerResult, IpcMessage};
use crate::state::AppState;
use crate::sync::{Job, SyncEngine, overview};
use crate::{launch, window};

/// Lets the reply to `quit` reach the sender before the app exits.
const QUIT_DELAY: Duration = Duration::from_millis(200);

fn json(value: impl Serialize) -> HandlerResult {
    serde_json::to_value(value).map_err(|e| AppError::from(e).into())
}

async fn status_report(app: &AppHandle) -> Result<StatusReport, AppError> {
    let sync = app.state::<SyncEngine>().state();
    Ok(StatusReport {
        version: app.package_info().version.to_string(),
        window_visible: window::main_visible(app),
        sync: overview(&app.state::<AppState>().db, sync).await?,
        place: places::place(app).await?,
    })
}

fn quit_after_reply(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(QUIT_DELAY).await;
        app.exit(0);
    });
}

/// Handles one message from another process and returns what its sender prints.
pub async fn handle(app: AppHandle, message: IpcMessage) -> HandlerResult {
    log::info!("Received {} from another process", message.kind());
    match message {
        IpcMessage::Show { focus } => {
            if focus {
                window::show_main(&app);
            } else {
                window::show_main_in_background(&app);
            }
            Ok(serde_json::Value::Null)
        }
        IpcMessage::Hide => {
            window::hide_main(&app);
            Ok(serde_json::Value::Null)
        }
        IpcMessage::Quit => {
            quit_after_reply(app);
            Ok(serde_json::Value::Null)
        }
        IpcMessage::Status => json(status_report(&app).await?),
        IpcMessage::Sync { full } => {
            app.state::<SyncEngine>()
                .queue(if full { Job::Full } else { Job::Delta });
            Ok(serde_json::Value::Null)
        }
        IpcMessage::Spaces => json(places::spaces(&app).await?),
        IpcMessage::Filters { space } => json(places::filters(&app, space).await?),
        IpcMessage::Views { level } => json(places::views(&app, level).await?),
        IpcMessage::Go { level, view } => json(places::go(&app, level, view).await?),
        IpcMessage::Open { key } => {
            places::open_issue(&app, key).await?;
            Ok(serde_json::Value::Null)
        }
        IpcMessage::Me { on } => {
            places::only_mine(&app, on);
            Ok(serde_json::Value::Null)
        }
        IpcMessage::Layout { layout } => {
            places::set_layout(&app, layout).await?;
            Ok(serde_json::Value::Null)
        }
        IpcMessage::Search { text } => json(places::search(&app, text).await?),
        IpcMessage::ResetCache => {
            app.state::<AppState>().db.reset_cache().await?;
            log::info!("Cache reset");
            app.state::<SyncEngine>().queue(Job::Full);
            json("The cache is empty; a full sync fetches your issues again.")
        }
        #[cfg(debug_assertions)]
        IpcMessage::DevDrift => json(crate::dev::drift()?),
        #[cfg(debug_assertions)]
        IpcMessage::DevSeedSynthetic { count } => {
            json(crate::dev::seed_synthetic(&app, count).await?)
        }
        launch @ (IpcMessage::Activate { .. } | IpcMessage::Error { .. }) => {
            launch::queue(&app, launch);
            Ok(serde_json::Value::Null)
        }
        #[cfg(debug_assertions)]
        IpcMessage::Eval { js } => {
            crate::dev::eval(&app, &js);
            Ok(serde_json::Value::Null)
        }
    }
}
