//! Reconciliation: finds issues the scope no longer returns. Jira has no feed of deleted issues, so the ids
//! are compared; a missing one is asked for directly, and what Jira doesn't return is gone.

use std::collections::HashSet;

use chrono::{Duration, Utc};

use super::full::{current_scope, refresh_fields, search_fields};
use super::map::format_utc;
use super::store::{live_issue_ids, mark_gone, purge_gone, record_reconcile, store_page};
use super::{SyncContext, SyncEvent};
use crate::error::AppResult;
use crate::jira::{BULK_FETCH_MAX, SearchRequest};

/// Gone issues stay reachable from pins, links and search this long, then leave the cache.
const KEEP_GONE: Duration = Duration::days(7);

pub async fn reconcile(
    ctx: &SyncContext,
    _emit: &(dyn Fn(SyncEvent) + Send + Sync),
) -> AppResult<()> {
    let scope = current_scope(ctx).await?;
    if scope.is_empty() {
        return Ok(());
    }
    let client = ctx.access.client(&ctx.settings.get().connection)?;
    let mut in_scope = HashSet::new();
    let mut pages = client.search_jql(SearchRequest {
        jql: scope,
        fields: vec!["updated".into()],
        expand: None,
    });
    while let Some(page) = pages.next_page().await {
        in_scope.extend(page?.issues.into_iter().map(|issue| issue.id));
    }
    let missing: Vec<String> = ctx
        .db
        .read(live_issue_ids)
        .await?
        .into_iter()
        .filter(|id| !in_scope.contains(id))
        .collect();
    if !missing.is_empty() {
        let ids = refresh_fields(ctx, &client).await?;
        let fields = search_fields(ctx, &ids).await?;
        for batch in missing.chunks(BULK_FETCH_MAX) {
            let found = client.bulk_fetch(batch, &fields).await?;
            let returned: HashSet<String> =
                found.issues.iter().map(|issue| issue.id.clone()).collect();
            let gone: Vec<String> = batch
                .iter()
                .filter(|id| !returned.contains(*id))
                .cloned()
                .collect();
            let ids = ids.clone();
            let now = format_utc(Utc::now());
            // Returned but out of scope (moved, finished long ago): stored as Jira has it, so views drop it.
            ctx.db
                .write(move |tx, changes| {
                    store_page(tx, changes, &found.issues, &ids, false, &now)?;
                    mark_gone(tx, changes, &gone, &now)
                })
                .await?;
        }
    }
    let now = Utc::now();
    let before = format_utc(now - KEEP_GONE);
    let stamp = format_utc(now);
    let purged = ctx
        .db
        .write(move |tx, _| {
            record_reconcile(tx, &stamp)?;
            purge_gone(tx, &before)
        })
        .await?;
    if purged > 0 {
        log::info!("Removed {purged} issues gone for more than a week");
    }
    Ok(())
}
