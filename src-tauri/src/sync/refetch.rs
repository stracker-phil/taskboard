//! One issue fetched again right after a write, so the cache holds what Jira made of it.

use chrono::Utc;
use rusqlite::Transaction;

use super::full::{refresh_fields, search_fields};
use super::map::format_utc;
use super::source::Source;
use super::store::store_page;
use crate::db::Db;
use crate::error::AppResult;

/// Fetches the issue with the sync's fields and stores it like a sync page, then runs `then` in the same
/// transaction. A missing issue stores nothing.
pub async fn refetch_issue(
    db: &Db,
    source: &dyn Source,
    id: &str,
    then: impl FnOnce(&Transaction) -> AppResult<()> + Send + 'static,
) -> AppResult<()> {
    let field_ids = refresh_fields(db, source).await?;
    let fields = search_fields(db, &field_ids).await?;
    let found = source.fetch(&[id.to_string()], &fields).await?;
    let now = format_utc(Utc::now());
    db.write(move |tx, changes| {
        store_page(tx, changes, &found.issues, &field_ids, false, &now)?;
        then(tx)
    })
    .await
}
