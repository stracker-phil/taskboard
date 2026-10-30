//! Opening the two files: pragmas, private file modes, migrations and the attached cache.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

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

/// Creates or migrates both files. Runs before any connection attaches the cache.
pub fn prepare(dir: &Path) -> AppResult<()> {
    let mut local = open_file(&local_path(dir))?;
    require_fts5(&local)?;
    migrate::migrate(&mut local, "local", LOCAL)?;
    carry_over_seen(&local, dir)?;
    prepare_cache(dir)
}

/// Copies the seen marks of a cache from before `local/0015` into the local file, before the cache's own
/// migration drops them.
fn carry_over_seen(local: &Connection, dir: &Path) -> AppResult<()> {
    if !cache_path(dir).exists() {
        return Ok(());
    }
    attach_cache(local, dir)?;
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

pub fn prepare_cache(dir: &Path) -> AppResult<()> {
    let mut cache = open_file(&cache_path(dir))?;
    migrate::migrate(&mut cache, "cache", CACHE)
}

/// A connection to the local file with the cache attached as `cache`.
pub fn open(dir: &Path) -> AppResult<Connection> {
    let conn = open_file(&local_path(dir))?;
    attach_cache(&conn, dir)?;
    Ok(conn)
}

pub fn attach_cache(conn: &Connection, dir: &Path) -> AppResult<()> {
    let path = cache_path(dir);
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

pub fn ensure_attached(conn: &Connection, dir: &Path) -> AppResult<()> {
    let attached: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM pragma_database_list WHERE name = 'cache')",
        [],
        |row| row.get(0),
    )?;
    if attached {
        Ok(())
    } else {
        attach_cache(conn, dir)
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

        prepare(dir.path()).unwrap();

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

        prepare(dir.path()).unwrap();

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

        prepare(dir.path()).unwrap();

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
        prepare(dir.path()).unwrap();

        prepare(dir.path()).unwrap();

        assert_eq!(
            read_seen(&local_path(dir.path())),
            [seen_row("20001", "2026-01-05T09:00:00.000Z", 0)]
        );
    }
}
