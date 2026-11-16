//! The Tauri commands for one issue.

use std::path::PathBuf;

use chrono::Utc;
use tauri::http::Response;
use tauri::{AppHandle, Manager, State};

use super::{ActivityItem, IssueDetail, Related, WebLink};
use crate::error::{AppError, AppResult};
use crate::lists::Viewer;
use crate::state::AppState;

fn viewer(state: &AppState) -> Viewer {
    Viewer::of(&state.settings.get().connection, Utc::now())
}

#[tauri::command]
pub async fn issue_get(state: State<'_, AppState>, id: String) -> AppResult<IssueDetail> {
    let mut detail = super::issue_get(&state.db, viewer(&state), id).await?;
    super::keep_site_urls(
        &mut detail.attachments,
        &state.settings.get().connection.site_url,
    );
    Ok(detail)
}

/// Comments, history and recorded field changes, newest first.
#[tauri::command]
pub async fn issue_activity(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<ActivityItem>> {
    super::issue_activity(&state.db, viewer(&state), id).await
}

/// Removes a gone issue from the cache.
#[tauri::command]
pub async fn issue_forget(state: State<'_, AppState>, id: String) -> AppResult<()> {
    super::issue_forget(&state.db, id).await
}

/// Children (an epic's issues, a task's subtasks) and links.
#[tauri::command]
pub async fn issue_related(state: State<'_, AppState>, id: String) -> AppResult<Related> {
    super::issue_related(&state.db, viewer(&state), id).await
}

/// Fetches the issue's comments when the cached ones may be behind; true when it fetched.
#[tauri::command]
pub async fn issue_sync_comments(state: State<'_, AppState>, id: String) -> AppResult<bool> {
    let connection = state.settings.get().connection;
    let source = crate::sync::source_for(&state.jira, &connection)?;
    super::issue_sync_comments(&state.db, source.as_ref(), id).await
}

/// The cached web links other than pull requests (`issue_pull_requests` has those).
#[tauri::command]
pub async fn issue_web_links(state: State<'_, AppState>, id: String) -> AppResult<Vec<WebLink>> {
    super::issue_other_web_links(&state.db, id).await
}

/// The issue as Markdown: front matter, description, rich fields, web links and comments, oldest first.
#[tauri::command]
pub async fn issue_markdown(state: State<'_, AppState>, id: String) -> AppResult<String> {
    let site_url = state.settings.get().connection.site_url;
    super::issue_markdown(&state.db, viewer(&state), site_url, id).await
}

/// Fetches the issue's web links; true when they changed.
#[tauri::command]
pub async fn issue_sync_web_links(state: State<'_, AppState>, id: String) -> AppResult<bool> {
    let connection = state.settings.get().connection;
    let source = crate::sync::source_for(&state.jira, &connection)?;
    super::issue_sync_web_links(&state.db, source.as_ref(), id, chrono::Utc::now()).await
}

/// Fetches the issue's history (the tracker's changelog); true when it changed.
#[tauri::command]
pub async fn issue_sync_history(state: State<'_, AppState>, id: String) -> AppResult<bool> {
    let connection = state.settings.get().connection;
    let source = crate::sync::source_for(&state.jira, &connection)?;
    super::issue_sync_history(&state.db, source.as_ref(), id, Utc::now()).await
}

/// Downloads the attachment unless it already is, then shows it in Quick Look.
#[tauri::command]
pub async fn attachment_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    issue_id: String,
    attachment_id: String,
) -> AppResult<()> {
    let (source, root) = attachment_source(&app, &state)?;
    let path =
        super::attachment_file(&state.db, source.as_ref(), &root, issue_id, attachment_id).await?;
    quick_look(&app, path).await
}

/// Saves the attachment to the Downloads folder under a free name and shows it in Finder (ADR 0075).
#[tauri::command]
pub async fn attachment_download(
    app: AppHandle,
    state: State<'_, AppState>,
    issue_id: String,
    attachment_id: String,
) -> AppResult<()> {
    let (source, root) = attachment_source(&app, &state)?;
    let cached =
        super::attachment_file(&state.db, source.as_ref(), &root, issue_id, attachment_id).await?;
    let saved = super::save_new(&downloads_dir(&app)?, &cached).await?;
    tauri_plugin_opener::reveal_item_in_dir(&saved)
        .map_err(|e| AppError::Io(format!("Finder couldn't show the file: {e}")))
}

/// Asks where to save the attachment, then saves it there; false when the owner cancels (ADR 0075).
#[tauri::command]
pub async fn attachment_save_as(
    app: AppHandle,
    state: State<'_, AppState>,
    issue_id: String,
    attachment_id: String,
) -> AppResult<bool> {
    let (source, root) = attachment_source(&app, &state)?;
    let name = super::attachment_name(&state.db, issue_id.clone(), attachment_id.clone()).await?;
    let Some(target) = save_panel(&app, name, downloads_dir(&app)?).await? else {
        return Ok(false);
    };
    let cached =
        super::attachment_file(&state.db, source.as_ref(), &root, issue_id, attachment_id).await?;
    super::save_to(&target, &cached).await?;
    Ok(true)
}

/// The system's save panel, as a sheet on the main window, starting in `folder` with `name`.
async fn save_panel(app: &AppHandle, name: String, folder: PathBuf) -> AppResult<Option<PathBuf>> {
    use tauri_plugin_dialog::DialogExt;
    let (done, picked) = tokio::sync::oneshot::channel();
    let mut panel = app
        .dialog()
        .file()
        .set_file_name(name)
        .set_directory(folder);
    if let Some(window) = app.get_webview_window("main") {
        panel = panel.set_parent(&window);
    }
    panel.save_file(move |path| {
        let _ = done.send(path);
    });
    let picked = picked
        .await
        .map_err(|_| AppError::Internal("The save panel didn't answer".into()))?;
    picked
        .map(|path| path.into_path())
        .transpose()
        .map_err(|e| AppError::InvalidInput(format!("Not a place to save to: {e}")))
}

/// Where attachments come from and where their copies are kept.
fn attachment_source(
    app: &AppHandle,
    state: &AppState,
) -> AppResult<(Box<dyn crate::sync::source::Source>, PathBuf)> {
    let source = crate::sync::source_for(&state.jira, &state.settings.get().connection)?;
    Ok((source, attachment_cache(app)?))
}

fn downloads_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .download_dir()
        .map_err(|e| AppError::Io(format!("The Downloads folder wasn't found: {e}")))
}

/// The `inline-image:` scheme's answer for `/<issue id>/<attachment id>` (ADR 0042).
pub async fn inline_image(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let answer = match super::image_path(path) {
        Some((issue_id, attachment_id)) => {
            let state = app.state::<AppState>();
            match (
                crate::sync::source_for(&state.jira, &state.settings.get().connection),
                attachment_cache(app),
            ) {
                (Ok(source), Ok(root)) => {
                    super::attachment_image(
                        &state.db,
                        source.as_ref(),
                        &root,
                        issue_id,
                        attachment_id,
                    )
                    .await
                }
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }
        None => Ok(None),
    };
    super::image_response(answer)
}

/// `<app cache>/attachments`.
pub fn attachment_cache(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Io(e.to_string()))?;
    Ok(super::cache_root(&dir))
}

#[cfg(target_os = "macos")]
async fn quick_look(app: &AppHandle, path: PathBuf) -> AppResult<()> {
    let (done, shown) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = done.send(crate::quicklook::show(&path));
    })
    .map_err(|e| AppError::Internal(format!("Quick Look couldn't start: {e}")))?;
    shown
        .await
        .map_err(|_| AppError::Internal("Quick Look didn't answer".into()))?
}

#[cfg(not(target_os = "macos"))]
async fn quick_look(_app: &AppHandle, _path: PathBuf) -> AppResult<()> {
    Err(AppError::Internal("Quick Look is macOS only".into()))
}
