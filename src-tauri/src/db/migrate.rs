//! Numbered SQL migrations per file, embedded in the binary. `PRAGMA <schema>.user_version` counts the ones
//! applied; a shipped migration is never edited, only followed by a new one.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub const LOCAL: &[&str] = &[
    include_str!("../../migrations/local/0001_init.sql"),
    include_str!("../../migrations/local/0002_tree.sql"),
    include_str!("../../migrations/local/0003_view_widths.sql"),
    include_str!("../../migrations/local/0004_view_wrap.sql"),
    include_str!("../../migrations/local/0005_view_collapsed.sql"),
    include_str!("../../migrations/local/0006_list_icon.sql"),
];
pub const CACHE: &[&str] = &[
    include_str!("../../migrations/cache/0001_init.sql"),
    include_str!("../../migrations/cache/0002_field_used.sql"),
    include_str!("../../migrations/cache/0003_source.sql"),
];

/// Brings the connection's file up to date in one transaction. A file migrated by a newer Taskboard is
/// refused, never touched.
pub fn migrate(conn: &mut Connection, name: &str, migrations: &[&str]) -> AppResult<()> {
    let version: usize = conn.query_row("PRAGMA user_version", [], |row| {
        row.get::<_, i64>(0).map(|v| v.max(0) as usize)
    })?;
    if version > migrations.len() {
        return Err(AppError::Db(format!(
            "The {name} database is from a newer Taskboard (version {version}, this one knows {})",
            migrations.len()
        )));
    }
    if version == migrations.len() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for sql in &migrations[version..] {
        tx.execute_batch(sql)?;
    }
    tx.execute_batch(&format!("PRAGMA user_version = {}", migrations.len()))?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn sources(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("SELECT source FROM {table} ORDER BY rowid"))
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    const SPACE: &str =
        "INSERT INTO space (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}')";
    const ISSUE: &str = "INSERT INTO issue (id, key, project_id, summary, issue_type, status_id, \
        status_category, created_at, updated_at, raw, synced_at) \
        VALUES ('20001', 'NW-1', '10001', 'Mount the column', 'Task', '1', 'new', \
        '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '{}', '2026-01-05T09:00:00Z')";
    const COMMENT: &str = "INSERT INTO comment (id, issue_id, created_at, updated_at) \
        VALUES ('30001', '20001', '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z')";

    #[test]
    fn a_fresh_cache_has_spaces_instead_of_projects_and_rows_default_to_jira() {
        let mut conn = Connection::open_in_memory().unwrap();

        migrate(&mut conn, "cache", CACHE).unwrap();

        let tables = tables(&conn);
        assert!(tables.contains(&"space".to_string()), "{tables:?}");
        assert!(!tables.contains(&"project".to_string()), "{tables:?}");
        for sql in [SPACE, ISSUE, COMMENT] {
            conn.execute(sql, []).unwrap();
        }
        for table in ["space", "issue", "comment"] {
            assert_eq!(sources(&conn, table), ["jira"], "{table}");
        }
    }

    #[test]
    fn a_cache_from_before_sources_keeps_its_rows_and_marks_them_jira() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "cache", &CACHE[..2]).unwrap();
        conn.execute(
            "INSERT INTO project (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}')",
            [],
        )
        .unwrap();
        conn.execute(ISSUE, []).unwrap();
        conn.execute(COMMENT, []).unwrap();

        migrate(&mut conn, "cache", CACHE).unwrap();

        let key: String = conn
            .query_row(
                "SELECT s.key FROM issue i JOIN space s ON s.id = i.project_id",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(key, "NW");
        for table in ["space", "issue", "comment"] {
            assert_eq!(sources(&conn, table), ["jira"], "{table}");
        }
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        let violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(violations, 0);
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CACHE.len() as i64);
    }
}
