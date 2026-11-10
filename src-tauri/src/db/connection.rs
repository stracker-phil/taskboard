//! Opening the two files: pragmas, private file modes, migrations and the attached cache.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use super::field_seed::seed_field_choices;
use super::migrate::{self, CACHE, LOCAL};
use crate::error::{AppError, AppResult};

pub const LOCAL_FILE: &str = "local.sqlite";
pub const CACHE_FILE: &str = "cache.sqlite";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub fn local_path(dir: &Path) -> PathBuf {
    dir.join(LOCAL_FILE)
}

pub fn cache_path(dir: &Path) -> PathBuf {
    dir.join(CACHE_FILE)
}

fn open_file(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;",
    )?;
    tune(&conn, "main")?;
    make_private(path)?;
    Ok(conn)
}

/// The files hold the owner's Jira data: readable by the user only, like the config dir around them.
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

/// Where the Jira mirror lives (ADR 0057).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheKind {
    /// `cache.sqlite` beside the local file.
    File,
    /// An in-memory database that is gone when Taskboard quits.
    Memory,
}

/// The attached mirror. A memory cache is shared by name between the connections of one `Db` and freed once
/// nothing holds it open, so it keeps its own connection for as long as it is in use.
pub enum Cache {
    File(PathBuf),
    Memory { uri: String, _keeper: Connection },
}

impl Cache {
    /// Creates or migrates the cache of `kind`. A memory cache starts empty and deletes the cache file.
    pub fn create(dir: &Path, kind: CacheKind) -> AppResult<Self> {
        match kind {
            CacheKind::File => {
                let path = cache_path(dir);
                migrate::migrate(&mut open_file(&path)?, "cache", CACHE)?;
                Ok(Self::File(path))
            }
            CacheKind::Memory => {
                remove_cache_files(dir)?;
                let uri = format!(
                    "file:/taskboard-cache-{}?vfs=memdb",
                    uuid::Uuid::new_v4().simple()
                );
                let mut keeper = Connection::open(&uri)?;
                keeper.busy_timeout(BUSY_TIMEOUT)?;
                migrate::migrate(&mut keeper, "cache", CACHE)?;
                Ok(Self::Memory {
                    uri,
                    _keeper: keeper,
                })
            }
        }
    }

    pub fn kind(&self) -> CacheKind {
        match self {
            Self::File(_) => CacheKind::File,
            Self::Memory { .. } => CacheKind::Memory,
        }
    }
}

/// Migrates the local file, then creates the cache. Runs before any connection attaches the cache.
pub fn prepare(dir: &Path, kind: CacheKind) -> AppResult<Cache> {
    let mut local = open_file(&local_path(dir))?;
    require_fts5(&local)?;
    migrate::migrate(&mut local, "local", LOCAL)?;
    carry_over_seen(&local, dir)?;
    seed_field_choices(&local, dir)?;
    Cache::create(dir, kind)
}

/// Copies the seen marks of a cache from before `local/0015` into the local file, before the cache's own
/// migration drops them.
fn carry_over_seen(local: &Connection, dir: &Path) -> AppResult<()> {
    if !cache_path(dir).exists() {
        return Ok(());
    }
    attach_cache_file(local, dir)?;
    let copied = copy_seen(local);
    detach_cache(local)?;
    copied
}

fn copy_seen(local: &Connection) -> AppResult<()> {
    let has_seen: bool = local.query_row(
        "SELECT EXISTS (SELECT 1 FROM cache.sqlite_master WHERE type = 'table' AND name = 'issue_seen')",
        [],
        |row| row.get(0),
    )?;
    if has_seen {
        local.execute(
            "INSERT OR IGNORE INTO main.issue_seen (issue_id, seen_at, unread)
             SELECT issue_id, seen_at, unread FROM cache.issue_seen",
            [],
        )?;
    }
    Ok(())
}

pub fn remove_cache_files(dir: &Path) -> AppResult<()> {
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
    match cache {
        Cache::File(path) => attach_file(conn, path),
        Cache::Memory { uri, .. } => {
            conn.execute("ATTACH DATABASE ?1 AS cache", [uri])?;
            Ok(())
        }
    }
}

/// Attaches `cache.sqlite` in `dir`, whichever kind of cache the app uses.
pub fn attach_cache_file(conn: &Connection, dir: &Path) -> AppResult<()> {
    attach_file(conn, &cache_path(dir))
}

fn attach_file(conn: &Connection, path: &Path) -> AppResult<()> {
    let path = path
        .to_str()
        .ok_or_else(|| AppError::Db(format!("{} is not a UTF-8 path", path.display())))?;
    conn.execute("ATTACH DATABASE ?1 AS cache", [path])?;
    conn.execute_batch("PRAGMA cache.journal_mode = WAL; PRAGMA cache.synchronous = NORMAL;")?;
    tune(conn, "cache")
}

/// A 16 MB page cache and up to 256 MB read through memory mapping per connection: a view reads most of the
/// mirror on every refresh.
fn tune(conn: &Connection, schema: &str) -> AppResult<()> {
    conn.execute_batch(&format!(
        "PRAGMA {schema}.cache_size = -16000; PRAGMA {schema}.mmap_size = 268435456;"
    ))?;
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

    type SeenRow = (String, String, i64);

    fn seen_row(id: &str, seen_at: &str, unread: i64) -> SeenRow {
        (id.to_string(), seen_at.to_string(), unread)
    }

    fn read_seen(path: &Path) -> Vec<SeenRow> {
        let conn = Connection::open(path).unwrap();
        let mut query = conn
            .prepare("SELECT issue_id, seen_at, unread FROM issue_seen ORDER BY issue_id")
            .unwrap();
        query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn has_table(path: &Path, name: &str) -> bool {
        Connection::open(path)
            .unwrap()
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [name],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn cache_before_seen_moved(dir: &Path, rows: &str) {
        let mut cache = Connection::open(cache_path(dir)).unwrap();
        cache.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
        migrate::migrate(&mut cache, "cache", &CACHE[..12]).unwrap();
        cache.execute_batch(rows).unwrap();
    }

    #[test]
    fn seen_marks_in_an_old_cache_move_to_the_local_file_with_their_unread_flag() {
        let dir = temp_config_dir();
        cache_before_seen_moved(
            dir.path(),
            "INSERT INTO issue_seen (issue_id, seen_at, unread) VALUES
               ('20001', '2026-01-05T09:00:00.000Z', 0),
               ('20002', '2026-01-06T10:30:00.000Z', 1)",
        );

        prepare(dir.path(), CacheKind::File).unwrap();

        assert_eq!(
            read_seen(&local_path(dir.path())),
            [
                seen_row("20001", "2026-01-05T09:00:00.000Z", 0),
                seen_row("20002", "2026-01-06T10:30:00.000Z", 1),
            ]
        );
        assert!(!has_table(&cache_path(dir.path()), "issue_seen"));
    }

    #[test]
    fn a_mark_already_in_the_local_file_wins_over_the_old_cache_copy() {
        let dir = temp_config_dir();
        cache_before_seen_moved(
            dir.path(),
            "INSERT INTO issue_seen (issue_id, seen_at, unread) VALUES
               ('20001', '2026-01-05T09:00:00.000Z', 0),
               ('20002', '2026-01-06T10:30:00.000Z', 0)",
        );
        let mut local = Connection::open(local_path(dir.path())).unwrap();
        migrate::migrate(&mut local, "local", LOCAL).unwrap();
        local
            .execute(
                "INSERT INTO issue_seen (issue_id, seen_at, unread)
                 VALUES ('20001', '2026-02-01T08:00:00.000Z', 1)",
                [],
            )
            .unwrap();
        drop(local);

        prepare(dir.path(), CacheKind::File).unwrap();

        assert_eq!(
            read_seen(&local_path(dir.path())),
            [
                seen_row("20001", "2026-02-01T08:00:00.000Z", 1),
                seen_row("20002", "2026-01-06T10:30:00.000Z", 0),
            ]
        );
    }

    #[test]
    fn a_fresh_folder_prepares_both_files_with_the_seen_table_only_in_the_local_one() {
        let dir = temp_config_dir();

        prepare(dir.path(), CacheKind::File).unwrap();

        assert_eq!(read_seen(&local_path(dir.path())), Vec::<SeenRow>::new());
        assert!(!has_table(&cache_path(dir.path()), "issue_seen"));
    }

    #[test]
    fn preparing_again_keeps_the_seen_marks() {
        let dir = temp_config_dir();
        cache_before_seen_moved(
            dir.path(),
            "INSERT INTO issue_seen (issue_id, seen_at) VALUES ('20001', '2026-01-05T09:00:00.000Z')",
        );
        prepare(dir.path(), CacheKind::File).unwrap();

        prepare(dir.path(), CacheKind::File).unwrap();

        assert_eq!(
            read_seen(&local_path(dir.path())),
            [seen_row("20001", "2026-01-05T09:00:00.000Z", 0)]
        );
    }
}
