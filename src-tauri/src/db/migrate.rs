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
    include_str!("../../migrations/local/0011_view_inline_labels.sql"),
    include_str!("../../migrations/local/0012_view_filter.sql"),
    include_str!("../../migrations/local/0013_view_subtasks.sql"),
    include_str!("../../migrations/local/0014_view_only_mine.sql"),
    include_str!("../../migrations/local/0015_issue_seen.sql"),
];
pub const CACHE: &[&str] = &[
    include_str!("../../migrations/cache/0001_init.sql"),
    include_str!("../../migrations/cache/0002_field_used.sql"),
    include_str!("../../migrations/cache/0003_source.sql"),
    include_str!("../../migrations/cache/0004_remote_link.sql"),
    include_str!("../../migrations/cache/0005_links_synced_at.sql"),
    include_str!("../../migrations/cache/0006_issue_unread.sql"),
    include_str!("../../migrations/cache/0007_issue_history.sql"),
    include_str!("../../migrations/cache/0008_avatar.sql"),
    include_str!("../../migrations/cache/0009_refetch_for_attachments.sql"),
    include_str!("../../migrations/cache/0010_comment_body_adf.sql"),
    include_str!("../../migrations/cache/0011_issue_body.sql"),
    include_str!("../../migrations/cache/0012_left_scope_at.sql"),
    include_str!("../../migrations/cache/0013_drop_issue_seen.sql"),
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
        status_category, created_at, updated_at, synced_at) \
        VALUES ('20001', 'NW-1', '10001', 'Mount the column', 'Task', '1', 'new', \
        '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z')";
    const ISSUE_BEFORE_BODIES: &str = "INSERT INTO issue (id, key, project_id, summary, issue_type, \
        status_id, status_category, created_at, updated_at, raw, synced_at) \
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
    fn views_from_before_the_saved_me_chip_start_with_it_off() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..13]).unwrap();
        conn.execute_batch(
            "INSERT INTO view (list_id, name, layout, group_by, sort_by, sort_order)
             VALUES (1, 'Board', 'board', 'status', 'updated', 0),
                    (1, 'List', 'list', 'status', 'updated', 0)",
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let mut query = conn
            .prepare("SELECT name, only_mine FROM view ORDER BY id")
            .unwrap();
        let rows: Vec<(String, bool)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for name in ["Board", "List"] {
            assert!(rows.contains(&(name.to_string(), false)), "{name}");
        }
        assert!(rows.iter().all(|(_, only_mine)| !only_mine), "{rows:?}");
    }

    #[test]
    fn the_attachments_migration_clears_the_delta_cursor_and_keeps_the_rest() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "cache", &CACHE[..8]).unwrap();
        conn.execute_batch(
            "INSERT INTO sync_state (scope_id, jql, last_delta_at, last_full_at)
             VALUES ('scope', 'project = NW', '2026-10-28T09:00:00.000Z', '2026-10-27T09:00:00.000Z'),
                    ('10001', 'project = NW', '2026-10-28T09:00:00.000Z', '2026-10-27T09:00:00.000Z')",
        )
        .unwrap();

        migrate(&mut conn, "cache", CACHE).unwrap();

        let mut query = conn
            .prepare("SELECT scope_id, last_delta_at, last_full_at FROM sync_state ORDER BY 1")
            .unwrap();
        let rows: Vec<(String, Option<String>, Option<String>)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows,
            [
                (
                    "10001".to_string(),
                    Some("2026-10-28T09:00:00.000Z".to_string()),
                    Some("2026-10-27T09:00:00.000Z".to_string())
                ),
                (
                    "scope".to_string(),
                    None,
                    Some("2026-10-27T09:00:00.000Z".to_string())
                ),
            ]
        );
    }

    fn issue_before_bodies(id: &str, key: &str, raw: &serde_json::Value, html: &str) -> String {
        format!(
            "INSERT INTO issue (id, key, project_id, summary, issue_type, status_id, status_category,
               created_at, updated_at, raw, description_html, description_adf, synced_at)
             VALUES ('{id}', '{key}', '10001', 'Mount the column', 'Task', '1', 'new',
               '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '{raw}', '{html}', '{{\"type\":\"doc\"}}',
               '2026-01-05T09:00:00Z')"
        )
    }

    fn cache_with_issue_bodies_migrated() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "cache", &CACHE[..10]).unwrap();
        let filled = serde_json::json!({
            "id": "20001",
            "key": "NW-1",
            "fields": {
                "summary": "Mount the column",
                "description": { "type": "doc", "version": 1, "content": [] },
                "attachment": [{ "id": "80001", "filename": "plan.png" }],
                "comment": {
                    "total": 1,
                    "comments": [{ "id": "30001", "updated": "2026-01-06T09:00:00Z", "body": "Long text" }]
                },
                "customfield_10001": "North",
                "customfield_10002": null,
                "customfield_10003": { "accountId": "u-1" }
            },
            "renderedFields": {
                "description": "<p>Hello</p>",
                "updated": "2 hours ago",
                "customfield_10001": "<p>North</p>",
                "customfield_10003": null
            }
        });
        let bare = serde_json::json!({
            "id": "20002",
            "key": "NW-2",
            "fields": { "summary": "Paint the wall", "customfield_10002": null }
        });
        for sql in [
            SPACE.to_string(),
            issue_before_bodies("20001", "NW-1", &filled, "<p>Hello</p>"),
            issue_before_bodies("20002", "NW-2", &bare, "<p>Plain</p>"),
            COMMENT.to_string(),
        ] {
            conn.execute(&sql, []).unwrap();
        }
        migrate(&mut conn, "cache", CACHE).unwrap();
        conn
    }

    #[test]
    fn the_issue_body_migration_moves_descriptions_and_a_trimmed_raw_into_issue_body() {
        let conn = cache_with_issue_bodies_migrated();

        let body = |id: &str| -> (Option<String>, Option<String>, serde_json::Value) {
            conn.query_row(
                "SELECT description_html, description_adf, raw FROM issue_body WHERE issue_id = ?1",
                [id],
                |r| {
                    let raw: String = r.get(2)?;
                    Ok((r.get(0)?, r.get(1)?, serde_json::from_str(&raw).unwrap()))
                },
            )
            .unwrap()
        };
        assert_eq!(
            body("20001"),
            (
                Some("<p>Hello</p>".to_string()),
                Some("{\"type\":\"doc\"}".to_string()),
                serde_json::json!({
                    "id": "20001",
                    "key": "NW-1",
                    "fields": {
                        "summary": "Mount the column",
                        "attachment": [{ "id": "80001", "filename": "plan.png" }]
                    },
                    "renderedFields": { "customfield_10001": "<p>North</p>" }
                })
            )
        );
        assert_eq!(
            body("20002").2,
            serde_json::json!({
                "id": "20002",
                "key": "NW-2",
                "fields": { "summary": "Paint the wall" },
                "renderedFields": {}
            })
        );
    }

    #[test]
    fn the_issue_body_migration_keeps_only_filled_custom_values_on_the_issue() {
        let conn = cache_with_issue_bodies_migrated();

        let custom = |id: &str| -> Option<serde_json::Value> {
            let text: Option<String> = conn
                .query_row("SELECT custom FROM issue WHERE id = ?1", [id], |r| r.get(0))
                .unwrap();
            text.map(|t| serde_json::from_str(&t).unwrap())
        };
        assert_eq!(
            custom("20001"),
            Some(serde_json::json!({
                "customfield_10001": "North",
                "customfield_10003": { "accountId": "u-1" }
            }))
        );
        assert_eq!(custom("20002"), None);
    }

    #[test]
    fn the_issue_body_migration_drops_the_big_columns_and_keeps_comments_and_cascades_bodies() {
        let conn = cache_with_issue_bodies_migrated();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();

        let columns: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('issue')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for dropped in ["raw", "description_html", "description_adf"] {
            assert!(!columns.contains(&dropped.to_string()), "{dropped}");
        }
        let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
        assert_eq!(count("SELECT COUNT(*) FROM comment"), 1);

        conn.execute("DELETE FROM issue WHERE id = '20001'", [])
            .unwrap();

        assert_eq!(count("SELECT COUNT(*) FROM issue_body"), 1);
        assert_eq!(
            count("SELECT COUNT(*) FROM issue_body WHERE issue_id = '20001'"),
            0
        );
    }

    #[test]
    fn existing_issues_start_without_a_left_scope_mark_after_the_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "cache", &CACHE[..11]).unwrap();
        for sql in [SPACE, ISSUE] {
            conn.execute(sql, []).unwrap();
        }

        migrate(&mut conn, "cache", CACHE).unwrap();

        let mark: Option<String> = conn
            .query_row("SELECT left_scope_at FROM issue", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mark, None);
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
        conn.execute(ISSUE_BEFORE_BODIES, []).unwrap();
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
