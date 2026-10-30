//! Reconciliation: finds issues the scope no longer returns. Jira has no feed of deleted issues, so the ids
//! are compared; a missing one is asked for directly, and what Jira doesn't return is gone. What Jira returns
//! outside the scope (done longer than the scope keeps, moved to an untracked project) stays cached and is
//! asked for again only after `RECHECK_LEFT_SCOPE`.

use std::collections::HashSet;

use chrono::{Duration, Utc};

use super::full::{current_scope, refresh_fields, search_fields};
use super::map::format_utc;
use super::retention::{
    live_issues, mark_gone, mark_left_scope, prune_seen_changes, purge_gone,
};
use super::store::{record_reconcile, store_page};
use super::{SyncContext, SyncEvent};
use crate::error::AppResult;
use crate::jira::{BULK_FETCH_MAX, SearchRequest};

/// Gone issues stay reachable from pins, links and search this long, then leave the cache.
const KEEP_GONE: Duration = Duration::days(7);
/// An issue found outside the scope is asked for again after this long, so one deleted since is noticed.
const RECHECK_LEFT_SCOPE: Duration = Duration::days(7);
/// Recorded changes the owner has seen are kept this long.
const KEEP_SEEN_CHANGES: Duration = Duration::days(90);

pub async fn reconcile(
    ctx: &SyncContext,
    _emit: &(dyn Fn(SyncEvent) + Send + Sync),
) -> AppResult<()> {
    let scope = current_scope(ctx).await?;
    if scope.is_empty() {
        return Ok(());
    }
    let source = ctx.source()?;
    let mut in_scope = HashSet::new();
    let mut pages = source.issues(SearchRequest {
        jql: scope,
        fields: vec!["updated".into()],
        expand: None,
    });
    while let Some(page) = pages.next_page().await {
        in_scope.extend(page?.issues.into_iter().map(|issue| issue.id));
    }
    let recheck_before = format_utc(Utc::now() - RECHECK_LEFT_SCOPE);
    let (inside, outside): (Vec<_>, Vec<_>) = ctx
        .db
        .read(live_issues)
        .await?
        .into_iter()
        .partition(|issue| in_scope.contains(&issue.id));
    let back: Vec<String> = inside
        .into_iter()
        .filter(|issue| issue.left_scope_at.is_some())
        .map(|issue| issue.id)
        .collect();
    let missing: Vec<String> = outside
        .into_iter()
        .filter(|issue| {
            issue
                .left_scope_at
                .as_ref()
                .is_none_or(|at| *at < recheck_before)
        })
        .map(|issue| issue.id)
        .collect();
    if !missing.is_empty() {
        let ids = refresh_fields(ctx, source.as_ref()).await?;
        let fields = search_fields(ctx, &ids).await?;
        for batch in missing.chunks(BULK_FETCH_MAX) {
            let found = source.fetch(batch, &fields).await?;
            let (returned, gone): (Vec<String>, Vec<String>) = batch
                .iter()
                .cloned()
                .partition(|id| found.issues.iter().any(|issue| issue.id == *id));
            let ids = ids.clone();
            let now = format_utc(Utc::now());
            // Returned but out of scope: stored as Jira has it, so views drop it, and not asked for hourly.
            ctx.db
                .write(move |tx, changes| {
                    store_page(tx, changes, &found.issues, &ids, false, &now)?;
                    mark_left_scope(tx, &returned, Some(&now))?;
                    mark_gone(tx, changes, &gone, &now)
                })
                .await?;
        }
    }
    let now = Utc::now();
    let gone_before = format_utc(now - KEEP_GONE);
    let changes_before = format_utc(now - KEEP_SEEN_CHANGES);
    let stamp = format_utc(now);
    let (purged, pruned) = ctx
        .db
        .write(move |tx, _| {
            record_reconcile(tx, &stamp)?;
            mark_left_scope(tx, &back, None)?;
            Ok((
                purge_gone(tx, &gone_before)?,
                prune_seen_changes(tx, &changes_before)?,
            ))
        })
        .await?;
    if purged > 0 {
        log::info!("Removed {purged} issues gone for more than a week");
    }
    if pruned > 0 {
        log::info!("Removed {pruned} seen changes older than 90 days");
    }
    Ok(())
}
