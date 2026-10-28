//! The delta sync: what changed since the last run, a 2-minute overlap included (upserts are idempotent).

use chrono::{DateTime, Duration, Utc};

use super::full::{current_scope, full_sync, refresh_fields, search_fields};
use super::map::format_utc;
use super::store::{last_delta_at, recently_updated, record_delta, store_page};
use super::{Job, SyncContext, SyncEvent, SyncProgress};
use crate::error::{AppError, AppResult};
use crate::issues::issue_sync_comments;
use crate::jira::{SearchRequest, jql};

/// Jira's JQL dates have minute precision; the overlap makes sure no change slips between two runs.
const OVERLAP: Duration = Duration::minutes(2);
/// After a delta, the comments of this many recently updated issues are fetched if they may be behind.
const PREFETCH_COMMENTS: usize = 20;

/// Without a cursor (no full sync yet) this runs the full sync instead.
pub async fn delta_sync(
    ctx: &SyncContext,
    emit: &(dyn Fn(SyncEvent) + Send + Sync),
) -> AppResult<()> {
    let Some(cursor) = ctx.db.read(last_delta_at).await? else {
        return full_sync(ctx, emit).await;
    };
    let scope = current_scope(ctx).await?;
    if scope.is_empty() {
        return Ok(());
    }
    let connection = ctx.settings.get().connection;
    let client = ctx.access.client(&connection)?;
    let ids = refresh_fields(ctx, &client).await?;
    let since = DateTime::parse_from_rfc3339(&cursor)
        .map_err(|e| AppError::Internal(format!("The delta cursor {cursor} isn't a time: {e}")))?
        .with_timezone(&Utc)
        - OVERLAP;
    // Jira reads JQL dates in the user's profile zone (`/myself`), not UTC and not the Mac's zone.
    let zone = connection.time_zone.as_deref().unwrap_or("UTC");
    let query = jql::order_by_updated(&jql::and_updated_since(
        &scope,
        &jql::date_literal(since, zone)?,
    ));
    let mut pages = client.search_jql(SearchRequest {
        jql: query,
        fields: search_fields(ctx, &ids).await?,
        expand: Some("renderedFields".into()),
    });
    let mut progress = SyncProgress {
        job: Job::Delta,
        project_id: String::new(),
        project_key: String::new(),
        pages_done: 0,
        fetched: 0,
        approx_total: None,
    };
    let mut newest: Option<String> = None;
    while let Some(page) = pages.next_page().await {
        let issues = page?.issues;
        let ids = ids.clone();
        let now = format_utc(Utc::now());
        let (page_newest, changed) = ctx
            .db
            .write(move |tx, changes| {
                let newest = store_page(tx, changes, &issues, &ids, false, &now)?;
                Ok((newest, changes.issues.len() as u32))
            })
            .await?;
        newest = newest.max(page_newest);
        progress.pages_done += 1;
        progress.fetched += changed;
        emit(SyncEvent::Progress(progress.clone()));
    }
    ctx.db
        .write(move |tx, _| record_delta(tx, newest.as_deref()))
        .await?;
    prefetch_comments(ctx).await;
    Ok(())
}

/// Best effort: a failure leaves the cached comments; the Issue view fetches again when opened.
async fn prefetch_comments(ctx: &SyncContext) {
    let Ok(ids) = ctx
        .db
        .read(|conn| recently_updated(conn, PREFETCH_COMMENTS))
        .await
    else {
        return;
    };
    let connection = ctx.settings.get().connection;
    for id in ids {
        if let Err(e) = issue_sync_comments(&ctx.db, &ctx.access, &connection, id).await {
            log::info!("Comment prefetch stopped: {e}");
            return;
        }
    }
}
