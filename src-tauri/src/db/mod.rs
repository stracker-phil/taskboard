//! The two SQLite files (ADR 0019): `local.sqlite`, the owner's own setup, with `cache.sqlite`, the Jira
//! mirror, attached as `cache`. One writer connection and a few readers; every call runs off the async
//! workers. All writes go through `Db::write`, which announces what changed after the commit.

mod changes;
mod coalesce;
mod connection;
mod migrate;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use rusqlite::{Connection, Transaction};

pub use changes::{Changes, IssuesChanged, LocalChanged, LocalPart};
pub use coalesce::{ANNOUNCE_EVERY, coalesce};
pub use connection::{CACHE_FILE, LOCAL_FILE};

use crate::error::{AppError, AppResult};
use crate::lock::MutexExt;

const READERS: usize = 2;

/// Receives what a committed write changed.
pub type Notifier = Box<dyn Fn(&Changes) + Send + Sync>;

#[derive(Clone)]
pub struct Db {
    inner: Arc<Inner>,
}

struct Inner {
    dir: PathBuf,
    writer: Mutex<Connection>,
    readers: Vec<Mutex<Connection>>,
    next_reader: AtomicUsize,
    notifier: OnceLock<Notifier>,
}

impl Db {
    /// Creates or migrates both files in `dir` and opens the connections.
    pub fn open(dir: &Path) -> AppResult<Self> {
        connection::prepare(dir)?;
        let readers = (0..READERS)
            .map(|_| connection::open(dir).map(Mutex::new))
            .collect::<AppResult<_>>()?;
        Ok(Self {
            inner: Arc::new(Inner {
                dir: dir.to_path_buf(),
                writer: Mutex::new(connection::open(dir)?),
                readers,
                next_reader: AtomicUsize::new(0),
                notifier: OnceLock::new(),
            }),
        })
    }

    /// Set once the app can emit events; writes before that announce nothing.
    pub fn set_notifier(&self, notifier: Notifier) {
        if self.inner.notifier.set(notifier).is_err() {
            log::warn!("The database notifier was already set");
        }
    }

    /// Runs a query on a reader connection, which sees both files.
    pub async fn read<T, F>(&self, f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> AppResult<T> + Send + 'static,
    {
        let inner = Arc::clone(&self.inner);
        blocking(move || {
            let index = inner.next_reader.fetch_add(1, Ordering::Relaxed) % inner.readers.len();
            f(&inner.readers[index].lock_unpoisoned())
        })
        .await
    }

    /// The one write path: runs `f` in a transaction on the writer, commits, then announces the changes `f`
    /// recorded. A failing `f` rolls back and announces nothing.
    pub async fn write<T, F>(&self, f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Transaction, &mut Changes) -> AppResult<T> + Send + 'static,
    {
        let inner = Arc::clone(&self.inner);
        blocking(move || {
            let mut changes = Changes::default();
            let result = {
                let mut conn = inner.writer.lock_unpoisoned();
                let tx = conn.transaction()?;
                let result = f(&tx, &mut changes)?;
                tx.commit()?;
                result
            };
            inner.announce(&changes);
            Ok(result)
        })
        .await
    }

    /// Deletes the Jira mirror and starts an empty one; the owner's own setup stays.
    pub async fn reset_cache(&self) -> AppResult<()> {
        let inner = Arc::clone(&self.inner);
        blocking(move || {
            let writer = inner.writer.lock_unpoisoned();
            let readers: Vec<_> = inner.readers.iter().map(|r| r.lock_unpoisoned()).collect();
            let conns: Vec<&Connection> = readers.iter().map(|r| &**r).chain([&*writer]).collect();
            let replaced = replace_cache(&conns, &inner.dir);
            // Even after a failure every connection gets a cache back, so later queries keep working.
            for conn in &conns {
                connection::ensure_attached(conn, &inner.dir)?;
            }
            replaced?;
            drop(readers);
            drop(writer);
            inner.announce(&Changes {
                cache_reset: true,
                ..Changes::default()
            });
            Ok(())
        })
        .await
    }
}

impl Inner {
    fn announce(&self, changes: &Changes) {
        if let Some(notify) = self.notifier.get()
            && !changes.is_empty()
        {
            notify(changes);
        }
    }
}

fn replace_cache(conns: &[&Connection], dir: &Path) -> AppResult<()> {
    for conn in conns {
        connection::detach_cache(conn)?;
    }
    remove_cache_files(dir)?;
    connection::prepare_cache(dir)
}

fn remove_cache_files(dir: &Path) -> AppResult<()> {
    for suffix in ["", "-wal", "-shm"] {
        let path = dir.join(format!("{CACHE_FILE}{suffix}"));
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(format!("A database task failed: {e}")))?
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::test_support::temp_config_dir;
    use std::collections::BTreeSet;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    const PROJECT: &str =
        "INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}')";
    const ISSUE: &str = "INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, \
        status_category, created_at, updated_at, raw, synced_at) \
        VALUES ('20001', 'NW-1', '10001', 'Mount the column', 'Task', '1', 'new', \
        '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '{}', '2026-01-05T09:00:00Z')";

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

    async fn count(db: &Db, sql: &'static str) -> i64 {
        db.read(move |conn| Ok(conn.query_row(sql, [], |row| row.get(0))?))
            .await
            .unwrap()
    }

    async fn text(db: &Db, sql: &'static str) -> String {
        db.read(move |conn| Ok(conn.query_row(sql, [], |row| row.get(0))?))
            .await
            .unwrap()
    }

    fn collect_into(sink: &Arc<Mutex<Vec<Changes>>>) -> Notifier {
        let sink = Arc::clone(sink);
        Box::new(move |changes| {
            sink.lock().unwrap().push(changes.clone());
        })
    }

    fn mode_of(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn prepare_future_file(path: &Path) {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch("PRAGMA user_version = 99").unwrap();
    }

    #[tokio::test]
    async fn open_creates_both_files_migrated_to_the_latest_version() {
        let (dir, db) = open_db();
        assert!(dir.path().join(LOCAL_FILE).is_file());
        assert!(dir.path().join(CACHE_FILE).is_file());
        assert_eq!(
            count(&db, "PRAGMA user_version").await,
            migrate::LOCAL.len() as i64
        );
        assert_eq!(
            count(&db, "PRAGMA cache.user_version").await,
            migrate::CACHE.len() as i64
        );
    }

    #[tokio::test]
    async fn connections_use_wal_and_enforce_foreign_keys() {
        let (_dir, db) = open_db();
        assert_eq!(text(&db, "PRAGMA journal_mode").await, "wal");
        assert_eq!(text(&db, "PRAGMA cache.journal_mode").await, "wal");
        assert_eq!(count(&db, "PRAGMA foreign_keys").await, 1);
        let orphan = db
            .write(|tx, _| {
                tx.execute(
                    "INSERT INTO view (list_id, name, layout, group_by, sort_by, sort_order) \
                     VALUES (999, 'Board', 'board', 'status', 'updated', 0)",
                    [],
                )?;
                Ok(())
            })
            .await;
        assert_eq!(orphan.unwrap_err().kind(), ErrorKind::Db);
    }

    #[tokio::test]
    async fn database_files_are_private_to_the_user() {
        let (dir, _db) = open_db();
        assert_eq!(mode_of(&dir.path().join(LOCAL_FILE)), 0o600);
        assert_eq!(mode_of(&dir.path().join(CACHE_FILE)), 0o600);
    }

    #[tokio::test]
    async fn reopening_the_same_folder_keeps_the_data() {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        exec(&db, PROJECT).await;
        exec(
            &db,
            "INSERT INTO space (project_id, name) VALUES ('10001', 'Shop floor')",
        )
        .await;
        drop(db);

        let db = Db::open(dir.path()).unwrap();
        assert_eq!(
            text(&db, "SELECT name FROM space WHERE project_id = '10001'").await,
            "Shop floor"
        );
        assert_eq!(text(&db, "SELECT name FROM cache.space").await, "Northwind");
    }

    #[test]
    fn a_local_file_from_a_newer_version_is_refused() {
        let dir = temp_config_dir();
        prepare_future_file(&dir.path().join(LOCAL_FILE));
        let err = Db::open(dir.path()).err().expect("open must fail");
        assert_eq!(err.kind(), ErrorKind::Db);
    }

    #[test]
    fn a_cache_file_from_a_newer_version_is_refused() {
        let dir = temp_config_dir();
        prepare_future_file(&dir.path().join(CACHE_FILE));
        let err = Db::open(dir.path()).err().expect("open must fail");
        assert_eq!(err.kind(), ErrorKind::Db);
    }

    #[tokio::test]
    async fn full_text_search_ignores_diacritics_and_keeps_the_text_for_snippets() {
        let (_dir, db) = open_db();
        exec(
            &db,
            "INSERT INTO cache.issue_fts (rowid, key, summary, description, comments) \
             VALUES (20001, 'NW-1', 'Säule montieren', '', '')",
        )
        .await;
        assert_eq!(
            count(
                &db,
                "SELECT rowid FROM cache.issue_fts WHERE issue_fts MATCH 'saule'"
            )
            .await,
            20001
        );
        assert_eq!(
            text(
                &db,
                "SELECT snippet(issue_fts, 1, '[', ']', '...', 8) FROM cache.issue_fts \
                 WHERE issue_fts MATCH 'saule'"
            )
            .await,
            "[Säule] montieren"
        );
    }

    #[tokio::test]
    async fn a_reader_can_join_local_rows_with_the_cache() {
        let (_dir, db) = open_db();
        exec(&db, PROJECT).await;
        exec(
            &db,
            "INSERT INTO space (project_id, name) VALUES ('10001', 'Shop floor')",
        )
        .await;
        let row: (String, String) = db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT local.name, tracked.key FROM main.space local \
                     JOIN cache.space tracked ON tracked.id = local.project_id",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(row, ("Shop floor".to_string(), "NW".to_string()));
    }

    #[tokio::test]
    async fn reset_cache_clears_the_mirror_and_keeps_the_local_setup() {
        let (_dir, db) = open_db();
        exec(&db, PROJECT).await;
        exec(&db, ISSUE).await;
        exec(&db, "INSERT INTO space (project_id) VALUES ('10001')").await;
        exec(
            &db,
            "INSERT INTO filter_list (kind, name) VALUES ('list', 'Open work')",
        )
        .await;
        exec(
            &db,
            "INSERT INTO view (list_id, name, layout, group_by, sort_by, sort_order) \
             VALUES ((SELECT id FROM filter_list WHERE name = 'Open work'), \
             'Board', 'board', 'status', 'updated', 0)",
        )
        .await;
        exec(
            &db,
            "INSERT INTO pin (target_kind, target_id, created_at) \
             VALUES ('issue', '20001', '2026-01-05T09:00:00Z')",
        )
        .await;
        let tables = [
            ("space", "SELECT COUNT(*) FROM space"),
            ("filter_list", "SELECT COUNT(*) FROM filter_list"),
            ("view", "SELECT COUNT(*) FROM view"),
            ("pin", "SELECT COUNT(*) FROM pin"),
        ];
        let mut before = Vec::new();
        for (_, sql) in tables {
            before.push(count(&db, sql).await);
        }

        db.reset_cache().await.unwrap();

        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.space").await, 0);
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.issue").await, 0);
        for ((table, sql), expected) in tables.into_iter().zip(before) {
            assert_eq!(count(&db, sql).await, expected, "{table}");
        }

        assert_eq!(
            count(&db, "PRAGMA cache.user_version").await,
            migrate::CACHE.len() as i64
        );
        exec(&db, PROJECT).await;
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.space").await, 1);
    }

    #[tokio::test]
    async fn reset_cache_announces_that_everything_may_have_changed() {
        let (_dir, db) = open_db();
        let sink = Arc::new(Mutex::new(Vec::new()));
        db.set_notifier(collect_into(&sink));

        db.reset_cache().await.unwrap();

        assert_eq!(
            *sink.lock().unwrap(),
            vec![Changes {
                cache_reset: true,
                ..Changes::default()
            }]
        );
    }

    #[tokio::test]
    async fn a_committed_write_announces_what_its_closure_recorded() {
        let (_dir, db) = open_db();
        let sink = Arc::new(Mutex::new(Vec::new()));
        db.set_notifier(collect_into(&sink));

        db.write(|tx, changes| {
            tx.execute_batch(PROJECT)?;
            changes.issue_changed("20002");
            changes.issue_changed("20001");
            changes.local_changed(LocalPart::Pins);
            changes.local_changed(LocalPart::Spaces);
            Ok(())
        })
        .await
        .unwrap();

        assert_eq!(
            *sink.lock().unwrap(),
            vec![Changes {
                issues: BTreeSet::from(["20001".to_string(), "20002".to_string()]),
                local: BTreeSet::from([LocalPart::Spaces, LocalPart::Pins]),
                ..Changes::default()
            }]
        );
    }

    #[tokio::test]
    async fn the_announcement_comes_after_the_commit() {
        let (dir, db) = open_db();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let cache_path = dir.path().join(CACHE_FILE);
        let sink = Arc::clone(&seen);
        db.set_notifier(Box::new(move |_| {
            let conn = rusqlite::Connection::open(&cache_path).unwrap();
            let projects: i64 = conn
                .query_row("SELECT COUNT(*) FROM space", [], |row| row.get(0))
                .unwrap();
            sink.lock().unwrap().push(projects);
        }));

        db.write(|tx, changes| {
            tx.execute_batch(PROJECT)?;
            changes.issue_changed("20001");
            Ok(())
        })
        .await
        .unwrap();

        assert_eq!(*seen.lock().unwrap(), vec![1]);
    }

    #[tokio::test]
    async fn a_failing_write_rolls_back_and_announces_nothing() {
        let (_dir, db) = open_db();
        let sink = Arc::new(Mutex::new(Vec::new()));
        db.set_notifier(collect_into(&sink));

        let result: AppResult<()> = db
            .write(|tx, changes| {
                tx.execute_batch(PROJECT)?;
                changes.issue_changed("20001");
                Err(AppError::InvalidInput("Nope".into()))
            })
            .await;

        assert_eq!(result.unwrap_err(), AppError::InvalidInput("Nope".into()));
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.space").await, 0);
        assert!(sink.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_write_that_records_nothing_announces_nothing() {
        let (_dir, db) = open_db();
        let sink = Arc::new(Mutex::new(Vec::new()));
        db.set_notifier(collect_into(&sink));

        exec(&db, PROJECT).await;

        assert!(sink.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn writes_before_a_notifier_is_set_still_commit() {
        let (_dir, db) = open_db();
        db.write(|tx, changes| {
            tx.execute_batch(PROJECT)?;
            changes.issue_changed("20001");
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.space").await, 1);
    }
}
