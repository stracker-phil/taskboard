//! The sync engine (`docs/sync.md`): one task owns all network work and mirrors the sync scope into the cache.

pub mod commands;
mod delta;
pub mod diff;
mod engine;
mod full;
pub mod map;
mod overview;
mod reconcile;
pub mod schedule;
pub mod store;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use engine::{Emit, SyncEngine, run_job};
pub use full::full_sync;
pub use overview::{ProjectCounts, SyncOverview, overview};

use crate::connection::JiraAccess;
use crate::db::Db;
use crate::error::ErrorKind;
use crate::store::settings::SettingsStore;

/// What a sync job needs; cheap to clone, so the engine's task owns its own copy.
#[derive(Clone)]
pub struct SyncContext {
    pub settings: SettingsStore,
    pub access: JiraAccess,
    pub db: Db,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Job {
    Full,
    Delta,
    Reconcile,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SyncState {
    /// `last_ok`: when the last job finished (UTC), if any.
    Idle {
        last_ok: Option<String>,
    },
    Running {
        job: Job,
    },
    Error {
        kind: ErrorKind,
        message: String,
    },
    /// Jira couldn't be reached; the next run tries again.
    Offline {
        since: String,
    },
    /// Jira refused the credentials; nothing runs until the connection is saved again.
    Paused {
        message: String,
    },
}

/// After every page of a full or delta sync; a delta counts changed issues in `fetched` and has no project.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub job: Job,
    pub project_id: String,
    pub project_key: String,
    pub pages_done: u32,
    pub fetched: u32,
    /// Jira's estimate for this project; `None` when it didn't give one.
    pub approx_total: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SyncEvent {
    State(SyncState),
    Progress(SyncProgress),
}
