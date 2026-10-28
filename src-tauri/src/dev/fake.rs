//! The fake Jira in the debug app: started at launch, connected on first launch, changed by `dev drift` and
//! `dev seed-synthetic`.

use std::sync::OnceLock;

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::connection::{self, ConnectionInput};
use crate::error::{AppError, AppResult};
use crate::jira::TokenKind;
use crate::lists::{self, SpaceUpdate};
use crate::state::AppState;
use crate::sync::{Job, SyncEngine};
use crate::test_support::demo::{self, ME_EMAIL};
use crate::test_support::fake_jira::{DEMO_TOKEN, FakeJira};

/// Where the debug build serves the demo Jira site.
pub const FAKE_JIRA_PORT: u16 = 47_470;

static FAKE: OnceLock<FakeJira> = OnceLock::new();

/// Serves the demo site on `FAKE_JIRA_PORT` (ADR 0022) until the app quits. Syncs once it is up: the sync
/// queued at launch may have run before.
pub fn start_fake_jira(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let now = chrono::Utc::now();
        let fake = match FakeJira::start_on(FAKE_JIRA_PORT, demo::demo_at(now), now).await {
            Ok(fake) => fake,
            Err(e) => {
                log::warn!("The fake Jira couldn't start on port {FAKE_JIRA_PORT}: {e}");
                return;
            }
        };
        let url = fake.url();
        log::info!("Fake Jira serves the demo site at {url}");
        let _ = FAKE.set(fake);
        if let Err(e) = connect_to_fake(&app, url.as_str()).await {
            log::warn!("Couldn't connect the debug app to the fake Jira: {e}");
        }
        app.state::<SyncEngine>().queue(Job::Full);
    });
}

async fn connect_to_fake(app: &AppHandle, url: &str) -> AppResult<()> {
    let state = app.state::<AppState>();
    if !state.settings.get().connection.site_url.is_empty() {
        return Ok(());
    }
    let input = ConnectionInput {
        site_url: url.into(),
        email: ME_EMAIL.into(),
        token_kind: TokenKind::Classic,
        token_expires: None,
        cloud_id: None,
    };
    let today = chrono::Local::now().date_naive();
    connection::save(
        &state.settings,
        &state.jira,
        input,
        Some(DEMO_TOKEN.into()),
        today,
    )
    .await?;
    let client = state.jira.client(&state.settings.get().connection)?;
    let projects = connection::refresh_projects(&state.db, &client).await?;
    for space in demo::demo_spaces().iter().filter(|s| s.tracked) {
        if let Some(project) = projects.iter().find(|p| p.key == space.key) {
            connection::fetch_versions(&state.db, &client, project.id.clone()).await?;
            connection::track_project(&state.db, project.id.clone(), true).await?;
            let look = SpaceUpdate {
                name: None,
                icon: Some(space.icon.to_string()),
                color: space
                    .color
                    .map(|token| token.trim_start_matches("space-tile-").to_string()),
                client: space.client.map(str::to_string),
            };
            lists::space_update(&state.db, project.id.clone(), look).await?;
        }
    }
    log::info!("Connected the debug app to the fake Jira");
    Ok(())
}

fn running() -> AppResult<&'static FakeJira> {
    FAKE.get()
        .ok_or_else(|| AppError::NotConnected("The fake Jira isn't running".into()))
}

/// `taskboard dev seed-synthetic <count>`: serves `count` generated issues instead of the demo tables, then
/// empties the cache and syncs them.
pub async fn seed_synthetic(app: &AppHandle, count: usize) -> AppResult<String> {
    let fake = running()?;
    fake.replace_data(demo::synthetic(count));
    app.state::<AppState>().db.reset_cache().await?;
    app.state::<SyncEngine>().queue(Job::Full);
    Ok(format!(
        "Serving {count} generated issues; syncing them now"
    ))
}

/// `taskboard dev drift`: changes the demo site the way a working day would, for the next sync to find.
pub fn drift() -> AppResult<String> {
    let fake = running()?;
    fake.set_now(chrono::Utc::now());
    let colleague = demo::demo()
        .users
        .into_iter()
        .find(|u| u["displayName"] == "Jonas Reiter")
        .ok_or_else(|| AppError::Internal("The demo site has no Jonas Reiter".into()))?;
    fake.add_comment(
        "NW-1042",
        colleague,
        "Pushed a fix for the totals, please have a look.",
    );
    let in_review = demo::demo()
        .statuses
        .into_iter()
        .find(|s| s["name"] == "In Review")
        .ok_or_else(|| AppError::Internal("The demo site has no In Review status".into()))?;
    fake.edit_issue("NW-1044", |fields| fields["status"] = in_review);
    fake.edit_issue("NW-1049", |fields| {
        fields["priority"] = json!({ "id": "1", "name": "Highest" })
    });
    let added = fake.add_issue("NW-1042", "NW", "Update the cookie banner copy");
    fake.delete_issue("NW-1055");
    Ok(format!(
        "Drifted the fake Jira: comment on NW-1042, NW-1044 in review, NW-1049 highest, new {added}, NW-1055 deleted"
    ))
}
