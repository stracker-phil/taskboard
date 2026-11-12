//! Keeps the query planner's statistics fresh and reports how much memory the Jira mirror takes.

use std::sync::Arc;

use rusqlite::Connection;

use super::{Db, blocking};
use crate::error::AppResult;
use crate::lock::MutexExt;

impl Db {
    /// Refreshes the planner's statistics where they went stale; cheap, for quit and after a full sync.
    pub async fn optimize(&self) -> AppResult<()> {
        let inner = Arc::clone(&self.inner);
        blocking(move || {
            Ok(inner
                .writer
                .lock_unpoisoned()
                .execute_batch("PRAGMA optimize; PRAGMA cache.optimize;")?)
        })
        .await
    }

    /// The mirror's size in memory.
    pub async fn cache_bytes(&self) -> AppResult<u64> {
        self.read(cache_bytes).await
    }
}

fn cache_bytes(conn: &Connection) -> AppResult<u64> {
    let pragma = |name: &str| -> AppResult<u64> {
        Ok(conn.query_row(&format!("PRAGMA cache.{name}"), [], |row| row.get(0))?)
    };
    Ok(pragma("page_count")? * pragma("page_size")?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use tempfile::TempDir;

    const PROJECT: &str =
        "INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}')";
    const MANY_ISSUES: &str = "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1500) \
        INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, \
        status_category, created_at, updated_at, synced_at) \
        SELECT 20000 + i, 'NW-' || i, '10001', hex(randomblob(300)), 'Task', '1', 'new', \
        '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z' FROM n";

    fn open_db() -> (TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        (dir, db)
    }

    async fn exec(db: &Db, sql: &'static str) {
        db.write(move |tx, _| {
            tx.execute_batch(sql)?;
            Ok(())
        })
        .await
        .unwrap();
    }

    async fn issue_count(db: &Db) -> i64 {
        db.read(
            |conn| Ok(conn.query_row("SELECT COUNT(*) FROM cache.issue", [], |row| row.get(0))?),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn an_empty_cache_already_takes_some_memory() {
        let (_dir, db) = open_db();

        assert!(db.cache_bytes().await.unwrap() > 0);
    }

    #[tokio::test]
    async fn the_cache_size_grows_when_rows_are_added() {
        let (_dir, db) = open_db();
        let empty = db.cache_bytes().await.unwrap();

        exec(&db, PROJECT).await;
        exec(&db, MANY_ISSUES).await;

        let filled = db.cache_bytes().await.unwrap();
        assert!(filled > empty, "{empty} -> {filled}");
    }

    #[tokio::test]
    async fn optimizing_succeeds_and_keeps_the_data() {
        let (_dir, db) = open_db();
        exec(&db, PROJECT).await;
        exec(&db, MANY_ISSUES).await;

        db.optimize().await.unwrap();

        assert_eq!(issue_count(&db).await, 1500);
    }
}
