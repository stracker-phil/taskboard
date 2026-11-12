//! Opening the local file (pragmas, private file mode, migrations) and the in-memory Jira mirror attached to it.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use super::migrate::{self, CACHE, LOCAL};
use crate::error::{AppError, AppResult};

pub const LOCAL_FILE: &str = "local.sqlite";
/// A mirror file left by earlier versions; deleted at launch.
pub const CACHE_FILE: &str = "cache.sqlite";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub fn local_path(dir: &Path) -> PathBuf {
    dir.join(LOCAL_FILE)
}

fn open_file(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;",
    )?;
    tune(&conn)?;
    make_private(path)?;
    Ok(conn)
}

/// Readable by the user only, like the config dir around it.
fn make_private(path: &Path) -> AppResult<()> {
    for suffix in ["", "-wal", "-shm"] {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        let file = PathBuf::from(name);
        if file.exists() {
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

/// The Jira mirror (ADR 0066): an in-memory database, gone when Taskboard quits. It is shared by name between
/// the connections of one `Db` and freed once nothing holds it open, so it keeps a connection of its own.
pub struct Cache {
    uri: String,
    _keeper: Connection,
}

impl Cache {
    /// An empty, migrated mirror.
    pub fn create() -> AppResult<Self> {
        let uri = format!(
            "file:/taskboard-cache-{}?vfs=memdb",
            uuid::Uuid::new_v4().simple()
        );
        let mut keeper = Connection::open(&uri)?;
        keeper.busy_timeout(BUSY_TIMEOUT)?;
        migrate::migrate(&mut keeper, "cache", CACHE)?;
        Ok(Self {
            uri,
            _keeper: keeper,
        })
    }
}

/// Runs before any connection attaches the cache; also removes a mirror file an earlier version left behind.
pub fn prepare(dir: &Path) -> AppResult<Cache> {
    let mut local = open_file(&local_path(dir))?;
    require_fts5(&local)?;
    migrate::migrate(&mut local, "local", LOCAL)?;
    remove_cache_files(dir)?;
    Cache::create()
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

/// A connection to the local file with `cache` attached.
pub fn open(dir: &Path, cache: &Cache) -> AppResult<Connection> {
    let conn = open_file(&local_path(dir))?;
    attach_cache(&conn, cache)?;
    Ok(conn)
}

pub fn attach_cache(conn: &Connection, cache: &Cache) -> AppResult<()> {
    conn.execute("ATTACH DATABASE ?1 AS cache", [&cache.uri])?;
    Ok(())
}

/// A 16 MB page cache and up to 256 MB read through memory mapping per connection.
fn tune(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA cache_size = -16000; PRAGMA mmap_size = 268435456;")?;
    Ok(())
}

pub fn ensure_attached(conn: &Connection, cache: &Cache) -> AppResult<()> {
    let attached: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM pragma_database_list WHERE name = 'cache')",
        [],
        |row| row.get(0),
    )?;
    if attached {
        Ok(())
    } else {
        attach_cache(conn, cache)
    }
}

pub fn detach_cache(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("DETACH DATABASE cache")?;
    Ok(())
}

/// The bundled SQLite has FTS5; a build without it can't search, so it refuses to start.
fn require_fts5(conn: &Connection) -> AppResult<()> {
    let mut stmt = conn.prepare("PRAGMA compile_options")?;
    let has_fts5 = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(Result::ok)
        .any(|option| option == "ENABLE_FTS5");
    if has_fts5 {
        Ok(())
    } else {
        Err(AppError::Internal("This SQLite build has no FTS5".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;

    fn table_names(conn: &Connection, schema: &str) -> Vec<String> {
        let mut query = conn
            .prepare(&format!(
                "SELECT name FROM {schema}.sqlite_master WHERE type = 'table'"
            ))
            .unwrap();
        query
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn a_fresh_folder_gets_a_local_file_and_no_cache_file() {
        let dir = temp_config_dir();

        prepare(dir.path()).unwrap();

        assert!(local_path(dir.path()).exists());
        assert!(!dir.path().join(CACHE_FILE).exists());
    }

    #[test]
    fn the_seen_table_lives_in_the_local_file_and_not_in_the_cache() {
        let dir = temp_config_dir();
        let cache = prepare(dir.path()).unwrap();

        let conn = open(dir.path(), &cache).unwrap();

        assert!(table_names(&conn, "main").contains(&"issue_seen".to_string()));
        assert!(!table_names(&conn, "cache").contains(&"issue_seen".to_string()));
    }

    #[test]
    fn preparing_deletes_a_leftover_cache_file_and_its_companions() {
        let dir = temp_config_dir();
        for suffix in ["", "-wal", "-shm"] {
            std::fs::write(dir.path().join(format!("{CACHE_FILE}{suffix}")), b"old").unwrap();
        }

        prepare(dir.path()).unwrap();

        for suffix in ["", "-wal", "-shm"] {
            assert!(
                !dir.path().join(format!("{CACHE_FILE}{suffix}")).exists(),
                "{suffix}"
            );
        }
    }

    #[test]
    fn every_prepare_returns_a_new_empty_cache_and_keeps_the_seen_marks() {
        let dir = temp_config_dir();
        let first = prepare(dir.path()).unwrap();
        let conn = open(dir.path(), &first).unwrap();
        conn.execute(
            "INSERT INTO issue_seen (issue_id, seen_at) VALUES ('20001', '2026-01-05T09:00:00.000Z')",
            [],
        )
        .unwrap();
        drop(conn);

        let second = prepare(dir.path()).unwrap();

        let conn = open(dir.path(), &second).unwrap();
        let seen: i64 = conn
            .query_row("SELECT COUNT(*) FROM issue_seen", [], |r| r.get(0))
            .unwrap();
        let issues: i64 = conn
            .query_row("SELECT COUNT(*) FROM cache.issue", [], |r| r.get(0))
            .unwrap();
        assert_eq!((seen, issues), (1, 0));
        assert_ne!(first.uri, second.uri);
    }
}
