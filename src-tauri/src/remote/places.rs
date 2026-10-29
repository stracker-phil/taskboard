//! The main window's places for the command line: listing and opening levels and views, opening an issue.

use chrono::Utc;
use rusqlite::OptionalExtension;
use tauri::{AppHandle, Emitter, Manager};

use super::levels::{find_level, find_view, level_label, level_rows, space_rows, view_rows};
use crate::cli::rows::{IssueRow, LevelRow, PlaceReport, SpaceRow, ViewsReport};
use crate::error::{AppError, AppResult};
use crate::events::NAV_CHANGED_EVENT;
use crate::launch::{self, LaunchEvent};
use crate::lists::{SidebarTree, Viewer, sidebar_tree};
use crate::nav::{Location, Nav};
use crate::search::palette_query;
use crate::state::AppState;
use crate::views::{Layout, list_views, view_set_layout};

async fn tree(state: &AppState) -> AppResult<SidebarTree> {
    let viewer = Viewer::of(&state.settings.get().connection, Utc::now());
    sidebar_tree(&state.db, viewer).await
}

async fn here(state: &AppState) -> AppResult<Option<Location>> {
    Ok(state.nav.current(&state.db).await?.location)
}

fn level(tree: &SidebarTree, word: &str) -> AppResult<i64> {
    find_level(tree, word).map_err(AppError::NotFound)
}

pub async fn spaces(app: &AppHandle) -> AppResult<Vec<SpaceRow>> {
    let state = app.state::<AppState>();
    let current = here(&state).await?.map(|at| at.list_id);
    Ok(space_rows(&tree(&state).await?, current))
}

pub async fn filters(app: &AppHandle, space: Option<String>) -> AppResult<Vec<LevelRow>> {
    let state = app.state::<AppState>();
    let tree = tree(&state).await?;
    let space = match space {
        Some(word) => {
            let id = level(&tree, &word)?;
            if !tree.spaces.iter().any(|s| s.list_id == id) {
                return Err(AppError::InvalidInput(format!("“{word}” isn't a Space.")));
            }
            Some(id)
        }
        None => None,
    };
    let current = here(&state).await?.map(|at| at.list_id);
    Ok(level_rows(&tree, space, current))
}

pub async fn views(app: &AppHandle, word: Option<String>) -> AppResult<ViewsReport> {
    let state = app.state::<AppState>();
    let tree = tree(&state).await?;
    let at = here(&state).await?;
    let list_id = match (word, at) {
        (Some(word), _) => level(&tree, &word)?,
        (None, Some(at)) => at.list_id,
        (None, None) => return Err(AppError::NotFound("No level is open.".into())),
    };
    let views = state.db.read(move |conn| list_views(conn, list_id)).await?;
    let current = at.filter(|at| at.list_id == list_id).map(|at| at.view_id);
    Ok(ViewsReport {
        level: level_label(&tree, list_id).unwrap_or_default(),
        views: view_rows(&views, current),
    })
}

/// The current place in words; none while no level exists.
pub async fn place(app: &AppHandle) -> AppResult<Option<PlaceReport>> {
    let state = app.state::<AppState>();
    let Some(at) = here(&state).await? else {
        return Ok(None);
    };
    let tree = tree(&state).await?;
    let views = state
        .db
        .read(move |conn| list_views(conn, at.list_id))
        .await?;
    Ok(Some(PlaceReport {
        list_id: at.list_id,
        level: level_label(&tree, at.list_id).unwrap_or_default(),
        view_id: at.view_id,
        view: views
            .iter()
            .find(|v| v.id == at.view_id)
            .map(|v| v.name.clone())
            .unwrap_or_default(),
    }))
}

/// Moves the main window and tells the page.
async fn go_to(app: &AppHandle, list_id: i64, view_id: Option<i64>) -> AppResult<Nav> {
    let state = app.state::<AppState>();
    let nav = state.nav.go(&state.db, list_id, view_id).await?;
    if let Err(e) = app.emit(NAV_CHANGED_EVENT, &nav) {
        log::warn!("Couldn't announce the new place: {e}");
    }
    crate::window::show_main(app);
    Ok(nav)
}

pub async fn go(
    app: &AppHandle,
    word: String,
    view: Option<String>,
) -> AppResult<Option<PlaceReport>> {
    let state = app.state::<AppState>();
    let list_id = level(&tree(&state).await?, &word)?;
    let view_id = match view {
        Some(view) => {
            let views = state.db.read(move |conn| list_views(conn, list_id)).await?;
            Some(find_view(&views, &view).map_err(AppError::NotFound)?)
        }
        None => None,
    };
    go_to(app, list_id, view_id).await?;
    place(app).await
}

/// In the full Issue view, over its Space's last view (where the row is selected), as the palette opens it.
pub async fn open_issue(app: &AppHandle, key: String) -> AppResult<()> {
    let state = app.state::<AppState>();
    let wanted = key.clone();
    let found = state
        .db
        .read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT id, project_id FROM cache.issue WHERE key = ?1",
                    [&wanted],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?)
        })
        .await?;
    let Some((id, project_id)) = found else {
        return Err(AppError::NotFound(format!(
            "{key} isn't in Taskboard; only issues of the tracked Spaces are."
        )));
    };
    let tree = tree(&state).await?;
    let list_id = tree
        .spaces
        .iter()
        .find(|s| s.project_id == project_id)
        .map_or(tree.everything.id, |s| s.list_id);
    let nav = go_to(app, list_id, None).await?;
    if let Some(origin) = nav.location {
        launch::queue_event(app, LaunchEvent::OpenIssue { id, origin });
    }
    Ok(())
}

pub fn only_mine(app: &AppHandle, on: Option<bool>) {
    launch::queue_event(app, LaunchEvent::OnlyMine { on });
}

pub async fn set_layout(app: &AppHandle, layout: Layout) -> AppResult<()> {
    let state = app.state::<AppState>();
    let at = here(&state)
        .await?
        .ok_or_else(|| AppError::NotFound("No view is open.".into()))?;
    view_set_layout(&state.db, at.view_id, layout).await?;
    crate::window::show_main(app);
    Ok(())
}

pub async fn search(app: &AppHandle, text: String) -> AppResult<Vec<IssueRow>> {
    let results = palette_query(&app.state::<AppState>().db, text).await?;
    Ok(results
        .issues
        .into_iter()
        .map(|hit| IssueRow {
            key: hit.key,
            status: hit.status_name,
            summary: hit.summary.into_iter().map(|part| part.text).collect(),
            gone: hit.gone,
        })
        .collect())
}
