//! The Tauri app: managed state, plugins, window and menu events, setup and the run loop.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

use crate::cli::LogLevel;
use crate::db::{ANNOUNCE_EVERY, Db, coalesce};
use crate::events::{
    ISSUE_DETAILS_CHANGED_EVENT, ISSUES_CHANGED_EVENT, LOCAL_CHANGED_EVENT, SYNC_PROGRESS_EVENT,
    SYNC_STATE_EVENT,
};
use crate::handlers::handlers;
use crate::ipc::IpcMessage;
use crate::state::AppState;
use crate::sync::schedule::{Focus, start_schedule};
use crate::sync::{SyncContext, SyncEngine, SyncEvent};
use crate::{bus, ipc, menu, remote, window};

const MAX_LOG_BYTES: u128 = 5 * 1024 * 1024;

pub fn start(
    state: AppState,
    socket: PathBuf,
    initial: IpcMessage,
    log_level: LogLevel,
    hidden: bool,
) {
    tauri::Builder::default()
        .manage(state)
        .manage(commands_bus())
        .plugin(log_plugin())
        .plugin(init_logging(log_level))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .on_menu_event(|app, event| menu::clicked(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(focused) = event
                && window.label() == "main"
                && let Some(focus) = window.app_handle().try_state::<Focus>()
            {
                focus.set(*focused);
            }
            if let WindowEvent::CloseRequested { api, .. } = event
                && window::close_requested(window.app_handle(), window.label())
            {
                api.prevent_close();
            }
        })
        .setup(move |app| {
            log::info!("Taskboard {} started", env!("CARGO_PKG_VERSION"));
            clear_attachments(app.handle());
            app.set_menu(menu::build(app.handle())?)?;
            let state = app.state::<AppState>();
            announce_db_changes(app.handle(), &state.db);
            give_spaces_their_lists(&state.db);
            carry_over_me_chips(app.handle(), &state);
            start_sync(app.handle(), &state);
            take_initial(app.handle(), &state, initial);
            #[cfg(debug_assertions)]
            apply_dev_frame(app);
            #[cfg(debug_assertions)]
            crate::dev::start_fake_jira(app.handle().clone());
            start_ipc(app.handle().clone(), &socket);
            listen_for_links(app.handle());
            // The window starts invisible (tauri.conf.json), so a hidden start never flashes it.
            if !hidden {
                window::show_main(app.handle());
            }
            Ok(())
        })
        .invoke_handler(handlers())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // Hiding the last window must not end the app; only Quit does (with an exit code).
            RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            RunEvent::Reopen { .. } => window::show_main(app),
            RunEvent::Exit => clear_attachments(app),
            _ => {}
        });
}

/// Downloaded attachments live only while the app runs.
fn clear_attachments(app: &AppHandle) {
    match crate::issues::commands::attachment_cache(app) {
        Ok(root) => crate::issues::clear_attachment_cache(&root),
        Err(e) => log::warn!("The attachment cache wasn't found: {e}"),
    }
}

/// A launch for the page waits for it; a request (`taskboard open`, `go`) that started the app runs now, and
/// its failure shows in the window, as no terminal waits for the answer.
fn take_initial(app: &AppHandle, state: &AppState, initial: IpcMessage) {
    if matches!(
        initial,
        IpcMessage::Activate { .. } | IpcMessage::Error { .. } | IpcMessage::Show { .. }
    ) {
        state.launches.push(initial);
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = remote::handle(app.clone(), initial).await {
            crate::launch::queue(&app, IpcMessage::Error { message: e.message });
        }
    });
}

/// A sync stores a page every few milliseconds; the page hears about them at most every `ANNOUNCE_EVERY`.
fn announce_db_changes(app: &AppHandle, db: &Db) {
    let app = app.clone();
    db.set_notifier(coalesce(
        ANNOUNCE_EVERY,
        Box::new(move |changes| {
            let issues = changes.issues_event();
            let details = changes.details_event();
            let sent = issues
                .map(|event| app.emit(ISSUES_CHANGED_EVENT, event))
                .into_iter()
                .chain(details.map(|event| app.emit(ISSUE_DETAILS_CHANGED_EVENT, event)))
                .chain(
                    changes
                        .local_event()
                        .map(|event| app.emit(LOCAL_CHANGED_EVENT, event)),
                );
            for result in sent {
                if let Err(e) = result {
                    log::warn!("Couldn't announce a data change: {e}");
                }
            }
        }),
    ));
}

/// Spaces tracked before this Taskboard knew filter lists get their defaults.
fn give_spaces_their_lists(db: &Db) {
    let db = db.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::lists::ensure_tracked_spaces(&db).await {
            log::warn!("Couldn't create the default filter lists: {e}");
        }
    });
}

/// Views whose place kept the Me chip on before views saved it (ADR 0038) save it now, once. Read before the
/// page can save a place, which would drop the old key.
fn carry_over_me_chips(app: &AppHandle, state: &AppState) {
    let ids = state.places.legacy_only_mine();
    if ids.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let carried = match crate::views::seed_only_mine(&state.db, ids).await {
            Ok(()) => state.places.forget_legacy_only_mine().await,
            Err(e) => Err(e),
        };
        if let Err(e) = carried {
            log::warn!("Couldn't carry over the views' Me chips: {e}");
        }
    });
}

/// The one sync task; its state and progress reach every window as events.
fn start_sync(app: &AppHandle, state: &AppState) {
    let handle = app.clone();
    let ctx = SyncContext {
        settings: state.settings.clone(),
        access: state.jira.clone(),
        db: state.db.clone(),
    };
    let ctx_for_schedule = ctx.clone();
    let engine = SyncEngine::start(
        ctx,
        Arc::new(move |event| {
            let sent = match event {
                SyncEvent::State(state) => handle.emit(SYNC_STATE_EVENT, state),
                SyncEvent::Progress(progress) => handle.emit(SYNC_PROGRESS_EVENT, progress),
            };
            if let Err(e) = sent {
                log::warn!("Couldn't announce the sync: {e}");
            }
        }),
    );
    let focus = Focus::default();
    start_schedule(ctx_for_schedule, engine.clone(), focus.clone());
    app.manage(focus);
    app.manage(engine);
}

fn commands_bus() -> bus::Commands {
    let mut commands = bus::CommandBus::default();
    bus::register(&mut commands);
    crate::nav::commands::register(&mut commands);
    bus::Commands(commands)
}

/// Links (`taskboard://…`) bring the window forward.
fn listen_for_links(app: &AppHandle) {
    let handle = app.clone();
    app.deep_link().on_open_url(move |_| {
        log::info!("Opened by a link");
        window::show_main(&handle);
    });
}

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
        ])
        .rotation_strategy(RotationStrategy::KeepSome(3))
        .max_file_size(MAX_LOG_BYTES)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        // The dispatch caps every later `log::set_max_level`, so it lets everything through;
        // `init_logging` sets the effective level.
        .level(log::LevelFilter::Trace)
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("wry", log::LevelFilter::Warn)
        .level_for("tauri_runtime_wry", log::LevelFilter::Info)
        .build()
}

/// Runs after `log_plugin`'s setup, which raises the max level to `Trace`, and applies the effective level.
fn init_logging(level: LogLevel) -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("log-level")
        .setup(move |_, _| {
            log::set_max_level(level.into());
            Ok(())
        })
        .build()
}

#[cfg(debug_assertions)]
fn apply_dev_frame(app: &tauri::App) {
    let (Some((x, y, w, h)), Some(window)) = (
        crate::dev::window_frame(),
        app.get_webview_window(window::MAIN),
    ) else {
        return;
    };
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    let _ = window.set_size(tauri::LogicalSize::new(w, h));
}

/// Without the socket the app still runs, only without single-instance handoff.
fn start_ipc(handle: AppHandle, socket: &Path) {
    let listener = match ipc::bind(socket) {
        Ok(listener) => listener,
        Err(e) => {
            log::warn!("Cannot listen on {}: {e}", socket.display());
            return;
        }
    };
    tauri::async_runtime::spawn(ipc::serve(listener, move |message| {
        remote::handle(handle.clone(), message)
    }));
}
