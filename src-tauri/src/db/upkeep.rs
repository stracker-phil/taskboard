//! Keeps the files lean: the query planner's statistics fresh, the write-ahead log short, and the cache compacted
//! once deletions left much of it empty.

use std::sync::Arc;

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use super::{Db, blocking};
use crate::error::AppResult;
use crate::lock::MutexExt;

/// Above this share of free pages, `tidy_cache` compacts the cache.
const COMPACT_ABOVE: f64 = 0.25;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct CacheSize {
    #[ts(type = "number")]
    pub bytes: u64,
    /// The share of the file that deletions left empty, 0 to 1.
    pub free: f64,
}

impl Db {
    /// Refreshes the planner's statistics where they went stale; cheap, for quit and after a full sync.
    pub async fn optimize(&self) -> AppResult<()> {
        self.on_writer(|conn| Ok(conn.execute_batch("PRAGMA optimize; PRAGMA cache.optimize;")?))
            .await
    }

    pub async fn cache_size(&self) -> AppResult<CacheSize> {
        self.read(cache_size).await
    }

    /// Rewrites the cache without its empty pages and empties its write-ahead log; returns the size after.
    pub async fn compact_cache(&self) -> AppResult<CacheSize> {
        self.on_writer(|conn| {
            compact(conn)?;
            cache_size(conn)
        })
        .await
    }

    /// After a full sync: compacts the cache when much of it is empty, else only trims its write-ahead log.
    pub async fn tidy_cache(&self) -> AppResult<()> {
        self.on_writer(|conn| {
            if cache_size(conn)?.free > COMPACT_ABOVE {
                compact(conn)
            } else {
                checkpoint(conn)
            }
        })
        .await?;
        self.optimize().await
    }

    /// Runs `f` on the writer outside a transaction, which `VACUUM` needs. Only for upkeep that changes no rows:
    /// it bypasses `Db::write`, so nothing is announced.
    async fn on_writer<T: Send + 'static>(
        &self,
        f: impl FnOnce(&Connection) -> AppResult<T> + Send + 'static,
    ) -> AppResult<T> {
        let inner = Arc::clone(&self.inner);
        blocking(move || f(&inner.writer.lock_unpoisoned())).await
    }
}

fn cache_size(conn: &Connection) -> AppResult<CacheSize> {
    let pragma = |name: &str| -> AppResult<u64> {
        Ok(conn.query_row(&format!("PRAGMA cache.{name}"), [], |row| row.get(0))?)
    };
    let (pages, free, page_size) = (
        pragma("page_count")?,
        pragma("freelist_count")?,
        pragma("page_size")?,
    );
    Ok(CacheSize {
        bytes: pages * page_size,
        free: free as f64 / pages.max(1) as f64,
    })
}

fn compact(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("VACUUM cache;")?;
    checkpoint(conn)
}

fn checkpoint(conn: &Connection) -> AppResult<()> {
    conn.query_row("PRAGMA cache.wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    Ok(())
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
    const KEEP_FIRST_THREE: &str =
        "DELETE FROM cache.issue WHERE key NOT IN ('NW-1', 'NW-2', 'NW-3')";

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

    async fn kept_keys(db: &Db) -> Vec<String> {
        db.read(|conn| {
            let mut stmt = conn.prepare("SELECT key FROM cache.issue ORDER BY key")?;
            let keys = stmt
                .query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            Ok(keys)
        })
        .await
        .unwrap()
    }

    async fn db_with_mostly_deleted_issues() -> (TempDir, Db) {
        let (dir, db) = open_db();
        exec(&db, PROJECT).await;
        exec(&db, MANY_ISSUES).await;
        exec(&db, KEEP_FIRST_THREE).await;
        (dir, db)
    }

    #[tokio::test]
    async fn a_fresh_cache_has_almost_no_free_pages() {
        let (_dir, db) = open_db();

        let size = db.cache_size().await.unwrap();

        assert!(size.bytes > 0);
        assert!(size.free < 0.1, "{size:?}");
    }

    #[tokio::test]
    async fn deleting_most_rows_leaves_the_cache_mostly_free() {
        let (_dir, db) = db_with_mostly_deleted_issues().await;

        let size = db.cache_size().await.unwrap();

        assert!(size.free > COMPACT_ABOVE, "{size:?}");
    }

    #[tokio::test]
    async fn compacting_shrinks_the_cache_and_keeps_its_rows() {
        let (_dir, db) = db_with_mostly_deleted_issues().await;
        let before = db.cache_size().await.unwrap();

        let after = db.compact_cache().await.unwrap();

        assert!(after.bytes < before.bytes, "{before:?} -> {after:?}");
        assert!(after.free < 0.05, "{after:?}");
        assert_eq!(db.cache_size().await.unwrap(), after);
        assert_eq!(kept_keys(&db).await, ["NW-1", "NW-2", "NW-3"]);
    }

    #[tokio::test]
    async fn tidying_compacts_a_cache_that_is_mostly_free() {
        let (_dir, db) = db_with_mostly_deleted_issues().await;
        let before = db.cache_size().await.unwrap();

        db.tidy_cache().await.unwrap();

        let after = db.cache_size().await.unwrap();
        assert!(after.bytes < before.bytes, "{before:?} -> {after:?}");
        assert!(after.free < 0.05, "{after:?}");
        assert_eq!(kept_keys(&db).await, ["NW-1", "NW-2", "NW-3"]);
    }

    #[tokio::test]
    async fn tidying_leaves_a_cache_with_few_free_pages_at_its_size() {
        let (_dir, db) = open_db();
        exec(&db, PROJECT).await;
        exec(&db, MANY_ISSUES).await;
        let before = db.cache_size().await.unwrap();

        db.tidy_cache().await.unwrap();

        let after = db.cache_size().await.unwrap();
        assert!(after.bytes >= before.bytes, "{before:?} -> {after:?}");
        assert_eq!(kept_keys(&db).await.len(), 1500);
    }

    #[tokio::test]
    async fn optimizing_succeeds_and_keeps_the_data() {
        let (_dir, db) = db_with_mostly_deleted_issues().await;

        db.optimize().await.unwrap();

        assert_eq!(kept_keys(&db).await, ["NW-1", "NW-2", "NW-3"]);
    }

    #[test]
    fn the_cache_size_serializes_with_camel_case_fields() {
        let size = CacheSize {
            bytes: 8192,
            free: 0.5,
        };

        assert_eq!(
            serde_json::to_value(size).unwrap(),
            serde_json::json!({ "bytes": 8192, "free": 0.5 })
        );
    }
}
