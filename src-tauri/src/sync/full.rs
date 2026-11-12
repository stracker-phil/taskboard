//! The full sync: metadata first, then the scope per tracked project (ADR 0023), page by page.

use chrono::Utc;

use super::map::FieldIds;
use super::own::{drop_own_changes, newest_change};
use super::source::Source;
use super::store::{
    Pass, fetched_fields, has_project_sync, page_pass, record_project_sync, record_scope_sync,
    store_fields, store_page, store_statuses, store_versions,
};
use super::{Job, SyncContext, SyncEvent, SyncProgress};
use crate::connection::{ProjectRow, store_spaces, sync_query, tracked_projects};
use crate::db::Db;
use crate::error::AppResult;
use crate::jira::dto::Myself;
use crate::jira::{SearchRequest, jql};
use crate::notify::rules::{resettle, settle};
use crate::store::settings::{Connection, Notifications};
use crate::time::format_utc;

/// Every issue field the cache maps, besides the site's sprint and epic color fields.
const SEARCH_FIELDS: &[&str] = &[
    "summary",
    "status",
    "issuetype",
    "priority",
    "assignee",
    "reporter",
    "parent",
    "labels",
    "duedate",
    "fixVersions",
    "created",
    "updated",
    "resolutiondate",
    "project",
    "comment",
    "description",
    "issuelinks",
    "attachment",
];

/// What every project of one run shares.
struct Run<'a> {
    ctx: &'a SyncContext,
    source: Box<dyn Source>,
    scope: String,
    fields: Vec<String>,
    ids: FieldIds,
    /// The connected account, whose own changes aren't news.
    me: Option<String>,
    notifications: Notifications,
    emit: &'a (dyn Fn(SyncEvent) + Send + Sync),
}

pub async fn full_sync(
    ctx: &SyncContext,
    emit: &(dyn Fn(SyncEvent) + Send + Sync),
) -> AppResult<()> {
    let settings = ctx.settings.get();
    let source = ctx.source()?;
    let (ids, tracked) = sync_metadata(ctx, source.as_ref()).await?;
    let keys: Vec<String> = tracked.iter().map(|p| p.key.clone()).collect();
    let scope = sync_query(&settings.sync, &keys);
    if scope.is_empty() {
        return Ok(());
    }
    let fields = search_fields(&ctx.db, &ids).await?;
    let after = ctx.db.read(newest_change).await?;
    let run = Run {
        ctx,
        source,
        scope,
        fields,
        ids,
        // Read again: `sync_metadata` just stored the account `/myself` named.
        me: ctx.settings.get().connection.account_id,
        notifications: settings.notifications,
        emit,
    };
    let mut newest: Option<String> = None;
    for project in &tracked {
        newest = newest.max(sync_project(&run, project).await?);
    }
    let now = format_utc(Utc::now());
    let scope = run.scope.clone();
    ctx.db
        .write(move |tx, _| record_scope_sync(tx, &scope, &now, newest.as_deref()))
        .await?;
    drop_own_changes(&ctx.db, run.source.as_ref(), run.me.as_deref(), after).await;
    Ok(())
}

/// The mapped fields, the sprint and epic color fields, and the custom fields `fetched_fields` returns.
pub(super) async fn search_fields(db: &Db, ids: &FieldIds) -> AppResult<Vec<String>> {
    let mut fields: Vec<String> = SEARCH_FIELDS.iter().map(|f| f.to_string()).collect();
    fields.extend(ids.sprint.iter().chain(&ids.epic_color).cloned());
    fields.extend(db.read(fetched_fields).await?);
    Ok(fields)
}

/// The site's field list, stored; returns its sprint and epic color fields.
pub(super) async fn refresh_fields(db: &Db, source: &dyn Source) -> AppResult<FieldIds> {
    let fields = source.fields().await?;
    db.write(move |tx, _| store_fields(tx, &fields)).await
}

/// The scope JQL over the tracked projects; empty when nothing is tracked.
pub(super) async fn current_scope(ctx: &SyncContext) -> AppResult<String> {
    let tracked = tracked_projects(&ctx.db).await?;
    if tracked.is_empty() {
        return Ok(String::new());
    }
    let keys: Vec<String> = tracked.iter().map(|p| p.key.clone()).collect();
    Ok(sync_query(&ctx.settings.get().sync, &keys))
}

/// Fields, projects, statuses, versions of the tracked projects, and who the user is.
async fn sync_metadata(
    ctx: &SyncContext,
    source: &dyn Source,
) -> AppResult<(FieldIds, Vec<ProjectRow>)> {
    let fields = source.fields().await?;
    let statuses = source.statuses().await?;
    store_spaces(&ctx.db, source.spaces().await?).await?;
    let tracked = tracked_projects(&ctx.db).await?;
    let mut versions = Vec::with_capacity(tracked.len());
    for project in &tracked {
        versions.push((project.id.clone(), source.versions(&project.key).await?));
    }
    let asked = ctx.settings.get().connection;
    remember_account(ctx, asked, source.myself().await?).await?;
    let ids = ctx
        .db
        .write(move |tx, _| {
            let ids = store_fields(tx, &fields)?;
            store_statuses(tx, &statuses)?;
            for (project_id, list) in &versions {
                store_versions(tx, project_id, list)?;
            }
            Ok(ids)
        })
        .await?;
    log::info!(
        "Jira fields: sprint {}, epic color {}",
        ids.sprint.as_deref().unwrap_or("none"),
        ids.epic_color.as_deref().unwrap_or("none")
    );
    Ok((ids, tracked))
}

/// Kept only if the connection wasn't changed while Jira answered. Another account settles the recorded changes
/// again: what is about "me" changed.
async fn remember_account(ctx: &SyncContext, asked: Connection, myself: Myself) -> AppResult<()> {
    let before = ctx.settings.get().connection.account_id;
    let saved = ctx
        .settings
        .edit(move |s| {
            if s.connection.same_account(&asked) {
                s.connection.account_id = Some(myself.account_id);
                s.connection.display_name = Some(myself.display_name);
                s.connection.time_zone = Some(myself.time_zone);
            }
        })
        .await?;
    if saved.connection.account_id != before {
        resettle(&ctx.db, &saved).await?;
    }
    Ok(())
}

/// One project's share of the scope, oldest change first; returns the newest `updated` stored.
async fn sync_project(run: &Run<'_>, project: &ProjectRow) -> AppResult<Option<String>> {
    let db = &run.ctx.db;
    let project_scope = jql::and_project(&run.scope, &project.key);
    let project_id = project.id.clone();
    let first = !db
        .read(move |conn| has_project_sync(conn, &project_id))
        .await?;
    let approx_total = run.source.count(&project_scope).await?;
    let mut progress = SyncProgress {
        job: Job::Full,
        project_id: project.id.clone(),
        project_key: project.key.clone(),
        pages_done: 0,
        fetched: 0,
        approx_total: Some(approx_total),
    };
    (run.emit)(SyncEvent::Progress(progress.clone()));
    let mut pages = run.source.issues(SearchRequest {
        jql: jql::order_by_updated(&project_scope),
        fields: run.fields.clone(),
        expand: Some("renderedFields".into()),
    });
    let mut newest: Option<String> = None;
    let mut pass = (!first).then_some(Pass::Later);
    while let Some(page) = pages.next_page().await {
        let issues = page?.issues;
        let count = issues.len() as u32;
        let ids = run.ids.clone();
        let me = run.me.clone();
        let notifications = run.notifications.clone();
        let now = format_utc(Utc::now());
        let (page_newest, page_pass) = db
            .write(move |tx, changes| {
                let pass = page_pass(tx, pass, &issues)?;
                let newest = store_page(tx, changes, &issues, &ids, pass, me.as_deref(), &now)?;
                settle(tx, changes, &notifications, me.as_deref(), false)?;
                Ok((newest, pass))
            })
            .await?;
        pass = Some(page_pass);
        newest = newest.max(page_newest);
        progress.pages_done += 1;
        progress.fetched += count;
        (run.emit)(SyncEvent::Progress(progress.clone()));
    }
    let now = format_utc(Utc::now());
    let project_id = project.id.clone();
    db.write(move |tx, _| record_project_sync(tx, &project_id, &project_scope, &now, approx_total))
        .await?;
    Ok(newest)
}
