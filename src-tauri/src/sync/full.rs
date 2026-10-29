//! The full sync: metadata first, then the scope per tracked project (ADR 0023), page by page.

use std::collections::BTreeSet;

use chrono::Utc;

use super::map::{FieldIds, format_utc};
use super::store::{
    has_project_sync, record_project_sync, record_scope_sync, shown_fields, store_field_usage,
    store_fields, store_page, store_statuses, store_versions,
};
use super::{Job, SyncContext, SyncEvent, SyncProgress};
use crate::connection::{ProjectRow, effective_scope, refresh_projects, tracked_projects};
use crate::error::AppResult;
use crate::jira::dto::Myself;
use crate::jira::{JiraClient, SearchRequest, jql};
use crate::store::settings::Connection;

/// Every issue field the cache maps, besides the site's sprint and epic colour fields.
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
];

/// What every project of one run shares.
struct Run<'a> {
    ctx: &'a SyncContext,
    client: JiraClient,
    scope: String,
    fields: Vec<String>,
    ids: FieldIds,
    emit: &'a (dyn Fn(SyncEvent) + Send + Sync),
}

pub async fn full_sync(
    ctx: &SyncContext,
    emit: &(dyn Fn(SyncEvent) + Send + Sync),
) -> AppResult<()> {
    let settings = ctx.settings.get();
    let client = ctx.access.client(&settings.connection)?;
    let (ids, tracked) = sync_metadata(ctx, &client).await?;
    let keys: Vec<String> = tracked.iter().map(|p| p.key.clone()).collect();
    let scope = effective_scope(&settings.sync, &keys).jql;
    if scope.is_empty() || tracked.is_empty() {
        return Ok(());
    }
    let fields = search_fields(ctx, &ids).await?;
    let run = Run {
        ctx,
        client,
        scope,
        fields,
        ids,
        emit,
    };
    let mut newest: Option<String> = None;
    for project in &tracked {
        newest = newest.max(sync_project(&run, project).await?);
    }
    let now = format_utc(Utc::now());
    let scope = run.scope;
    ctx.db
        .write(move |tx, _| record_scope_sync(tx, &scope, &now, newest.as_deref()))
        .await?;
    refresh_field_usage(ctx, &run.client, &tracked).await
}

/// What a search asks for: the mapped fields, the site's sprint and epic colour fields, and the custom fields
/// shown in the field grid or as a column.
pub(super) async fn search_fields(ctx: &SyncContext, ids: &FieldIds) -> AppResult<Vec<String>> {
    let mut fields: Vec<String> = SEARCH_FIELDS.iter().map(|f| f.to_string()).collect();
    fields.extend(ids.sprint.iter().chain(&ids.epic_color).cloned());
    fields.extend(ctx.db.read(shown_fields).await?);
    Ok(fields)
}

/// The site's field list, stored; returns its sprint and epic colour fields.
pub(super) async fn refresh_fields(ctx: &SyncContext, client: &JiraClient) -> AppResult<FieldIds> {
    let fields = client.fields().await?;
    ctx.db.write(move |tx, _| store_fields(tx, &fields)).await
}

/// The scope JQL over the tracked projects; empty when nothing is tracked.
pub(super) async fn current_scope(ctx: &SyncContext) -> AppResult<String> {
    let tracked = tracked_projects(&ctx.db).await?;
    if tracked.is_empty() {
        return Ok(String::new());
    }
    let keys: Vec<String> = tracked.iter().map(|p| p.key.clone()).collect();
    Ok(effective_scope(&ctx.settings.get().sync, &keys).jql)
}

/// Fields, projects, statuses, versions of the tracked projects, and who the user is.
async fn sync_metadata(
    ctx: &SyncContext,
    client: &JiraClient,
) -> AppResult<(FieldIds, Vec<ProjectRow>)> {
    let fields = client.fields().await?;
    let statuses = client.statuses().await?;
    refresh_projects(&ctx.db, client).await?;
    let tracked = tracked_projects(&ctx.db).await?;
    let mut versions = Vec::with_capacity(tracked.len());
    for project in &tracked {
        versions.push((project.id.clone(), client.versions(&project.key).await?));
    }
    let asked = ctx.settings.get().connection;
    remember_account(ctx, asked, client.myself().await?).await?;
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
        "Jira fields: sprint {}, epic colour {}",
        ids.sprint.as_deref().unwrap_or("none"),
        ids.epic_color.as_deref().unwrap_or("none")
    );
    Ok((ids, tracked))
}

/// Which custom fields the tracked projects fill; when a project can't be sampled, the last answer stays.
async fn refresh_field_usage(
    ctx: &SyncContext,
    client: &JiraClient,
    tracked: &[ProjectRow],
) -> AppResult<()> {
    let mut used = BTreeSet::new();
    for project in tracked {
        match client.used_custom_fields(&project.key).await {
            Ok(ids) => used.extend(ids),
            Err(e) => {
                log::warn!("Could not sample the fields of {}: {e}", project.key);
                return Ok(());
            }
        }
    }
    ctx.db
        .write(move |tx, _| store_field_usage(tx, &used))
        .await
}

/// Kept only if the connection wasn't changed while Jira answered.
async fn remember_account(ctx: &SyncContext, asked: Connection, myself: Myself) -> AppResult<()> {
    ctx.settings
        .edit(move |s| {
            if s.connection.same_account(&asked) {
                s.connection.account_id = Some(myself.account_id);
                s.connection.display_name = Some(myself.display_name);
                s.connection.time_zone = Some(myself.time_zone);
            }
        })
        .await?;
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
    let approx_total = run.client.approximate_count(&project_scope).await?;
    let mut progress = SyncProgress {
        job: Job::Full,
        project_id: project.id.clone(),
        project_key: project.key.clone(),
        pages_done: 0,
        fetched: 0,
        approx_total: Some(approx_total),
    };
    (run.emit)(SyncEvent::Progress(progress.clone()));
    let mut pages = run.client.search_jql(SearchRequest {
        jql: jql::order_by_updated(&project_scope),
        fields: run.fields.clone(),
        expand: Some("renderedFields".into()),
    });
    let mut newest: Option<String> = None;
    while let Some(page) = pages.next_page().await {
        let issues = page?.issues;
        let count = issues.len() as u32;
        let ids = run.ids.clone();
        let now = format_utc(Utc::now());
        let page_newest = db
            .write(move |tx, changes| store_page(tx, changes, &issues, &ids, first, &now))
            .await?;
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
