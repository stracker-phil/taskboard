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
    include_str!("../../migrations/local/0007_space_settings.sql"),
    include_str!("../../migrations/local/0008_unread_bookmarks.sql"),
    include_str!("../../migrations/local/0009_view_open_with.sql"),
    include_str!("../../migrations/local/0010_tile_looks.sql"),
];
pub const CACHE: &[&str] = &[
    include_str!("../../migrations/cache/0001_init.sql"),
    include_str!("../../migrations/cache/0002_field_used.sql"),
    include_str!("../../migrations/cache/0003_source.sql"),
    include_str!("../../migrations/cache/0004_remote_link.sql"),
    include_str!("../../migrations/cache/0005_links_synced_at.sql"),
    include_str!("../../migrations/cache/0006_issue_unread.sql"),
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
    fn app_wide_status_styles_become_every_spaces_override_when_space_settings_arrive() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..6]).unwrap();
        conn.execute_batch(
            "INSERT INTO space (project_id) VALUES ('10001'), ('10002');
             INSERT INTO status_style (status_id, style) VALUES ('s1', 'blocked'), ('s2', 'done');",
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let mut query = conn
            .prepare("SELECT project_id, status_id, style FROM space_status_style ORDER BY 1, 2")
            .unwrap();
        let rows: Vec<(String, String, String)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let expected: Vec<(String, String, String)> = [
            ("10001", "s1", "blocked"),
            ("10001", "s2", "done"),
            ("10002", "s1", "blocked"),
            ("10002", "s2", "done"),
        ]
        .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
        .to_vec();
        assert_eq!(rows, expected);
        assert!(!tables(&conn).contains(&"status_style".to_string()));
    }

    #[test]
    fn a_space_starts_with_all_layouts_and_every_field_visible_after_the_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..6]).unwrap();
        conn.execute("INSERT INTO space (project_id) VALUES ('10001')", [])
            .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let settings: (Option<String>, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT layouts, default_layout, default_group FROM space",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(settings, (None, None, None));
        let fields: i64 = conn
            .query_row("SELECT COUNT(*) FROM space_field", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fields, 0);
    }

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
    fn a_web_link_defaults_to_jira_and_goes_with_its_issue() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "cache", CACHE).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        for sql in [SPACE, ISSUE] {
            conn.execute(sql, []).unwrap();
        }
        conn.execute(
            "INSERT INTO remote_link (issue_id, id, position, url) \
             VALUES ('20001', '70001', 0, 'https://example.com/spec')",
            [],
        )
        .unwrap();

        assert_eq!(sources(&conn, "remote_link"), ["jira"]);

        conn.execute("DELETE FROM issue WHERE id = '20001'", [])
            .unwrap();

        assert_eq!(sources(&conn, "remote_link"), Vec::<String>::new());
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
