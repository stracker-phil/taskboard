//! The one sync task: runs queued jobs one at a time, coalesces duplicates and owns the sync state.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::sync::mpsc;

use super::full::full_sync;
use super::map::format_utc;
use super::store::last_full_at;
use super::{Job, SyncContext, SyncEvent, SyncState};
use crate::error::{AppError, AppResult};
use crate::lock::MutexExt;

pub type Emit = Arc<dyn Fn(SyncEvent) + Send + Sync>;

enum Command {
    Run(Job),
    ConnectionChanged,
}

/// The handle to the sync task; cheap to share through Tauri state.
pub struct SyncEngine {
    commands: mpsc::UnboundedSender<Command>,
    state: Arc<Mutex<SyncState>>,
}

impl SyncEngine {
    /// Starts the task. A cache that was never synced gets a full sync right away.
    pub fn start(ctx: SyncContext, emit: Emit) -> Self {
        let (commands, receiver) = mpsc::unbounded_channel();
        let state = Arc::new(Mutex::new(SyncState::Idle { last_ok: None }));
        tauri::async_runtime::spawn(run(ctx, emit, state.clone(), receiver));
        Self { commands, state }
    }

    /// Queues `job` unless the same job is already waiting.
    pub fn queue(&self, job: Job) {
        let _ = self.commands.send(Command::Run(job));
    }

    /// Lifts the pause after Jira refused the credentials.
    pub fn connection_changed(&self) {
        let _ = self.commands.send(Command::ConnectionChanged);
    }

    pub fn state(&self) -> SyncState {
        self.state.lock_unpoisoned().clone()
    }
}

async fn run(
    ctx: SyncContext,
    emit: Emit,
    state: Arc<Mutex<SyncState>>,
    mut receiver: mpsc::UnboundedReceiver<Command>,
) {
    // Every announced state is also what `SyncEngine::state` returns, including `running` from `run_job`.
    let announce = |event: SyncEvent| {
        if let SyncEvent::State(next) = &event {
            *state.lock_unpoisoned() = next.clone();
        }
        emit(event);
    };
    let set = |next: SyncState| announce(SyncEvent::State(next));
    let mut last_ok = last_full_sync(&ctx).await.unwrap_or_else(|e| {
        log::warn!("Couldn't read the last sync: {e}");
        None
    });
    let mut pending: VecDeque<Job> = VecDeque::new();
    if last_ok.is_none() {
        pending.push_back(Job::Full);
    }
    set(SyncState::Idle {
        last_ok: last_ok.clone(),
    });
    loop {
        if pending.is_empty() {
            let Some(command) = receiver.recv().await else {
                return;
            };
            apply(command, &mut pending, &state, &last_ok, &set);
        }
        while let Ok(command) = receiver.try_recv() {
            apply(command, &mut pending, &state, &last_ok, &set);
        }
        let Some(job) = pending.pop_front() else {
            continue;
        };
        if matches!(*state.lock_unpoisoned(), SyncState::Paused { .. }) {
            log::info!("Skipped a {job:?} sync: Jira refused the credentials");
            continue;
        }
        let next = run_job(&ctx, job, &announce, last_ok.clone()).await;
        if let SyncState::Idle { last_ok: ok } = &next {
            last_ok = ok.clone();
        }
        set(next);
    }
}

fn apply(
    command: Command,
    pending: &mut VecDeque<Job>,
    state: &Mutex<SyncState>,
    last_ok: &Option<String>,
    set: &impl Fn(SyncState),
) {
    match command {
        Command::Run(job) if !pending.contains(&job) => pending.push_back(job),
        Command::Run(_) => {}
        Command::ConnectionChanged => {
            let paused = matches!(*state.lock_unpoisoned(), SyncState::Paused { .. });
            if paused {
                set(SyncState::Idle {
                    last_ok: last_ok.clone(),
                });
            }
        }
    }
}

/// Runs one job and returns the state after it. Without a connection or a token nothing runs.
pub async fn run_job(
    ctx: &SyncContext,
    job: Job,
    emit: &(dyn Fn(SyncEvent) + Send + Sync),
    last_ok: Option<String>,
) -> SyncState {
    let connection = ctx.settings.get().connection;
    if connection.site().is_err() || !ctx.access.has_token(&connection.email) {
        log::info!("Skipped a {job:?} sync: Jira isn't connected");
        return SyncState::Idle { last_ok };
    }
    emit(SyncEvent::State(SyncState::Running { job }));
    let result = match job {
        Job::Full => full_sync(ctx, emit).await,
    };
    let now = format_utc(Utc::now());
    match result {
        Ok(()) => SyncState::Idle { last_ok: Some(now) },
        Err(AppError::Auth(message)) => {
            log::warn!("Sync paused: {message}");
            SyncState::Paused { message }
        }
        Err(AppError::Offline(message)) => {
            log::info!("Sync skipped, offline: {message}");
            SyncState::Offline { since: now }
        }
        Err(e) => {
            log::error!("Sync failed: {e}");
            SyncState::Error {
                kind: e.kind(),
                message: e.to_string(),
            }
        }
    }
}

async fn last_full_sync(ctx: &SyncContext) -> AppResult<Option<String>> {
    ctx.db.read(last_full_at).await
}
