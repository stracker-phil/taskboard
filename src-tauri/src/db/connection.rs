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
    prepare_cache(dir)
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
