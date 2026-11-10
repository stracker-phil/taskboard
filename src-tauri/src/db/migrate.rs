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
    include_str!("../../migrations/local/0016_space_member_off.sql"),
    include_str!("../../migrations/local/0017_space_member.sql"),
    include_str!("../../migrations/local/0018_drop_unread_list.sql"),
    include_str!("../../migrations/local/0019_pin_title_position.sql"),
    include_str!("../../migrations/local/0020_view_person.sql"),
    include_str!("../../migrations/local/0021_write_log.sql"),
    include_str!("../../migrations/local/0022_space_accent.sql"),
    include_str!("../../migrations/local/0023_view_me_rules.sql"),
    include_str!("../../migrations/local/0024_issue_activity.sql"),
    include_str!("../../migrations/local/0025_recently_viewed.sql"),
    include_str!("../../migrations/local/0026_status_style.sql"),
    include_str!("../../migrations/local/0027_field_choice.sql"),
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
    include_str!("../../migrations/cache/0014_pending_write.sql"),
    include_str!("../../migrations/cache/0015_drop_field_usage.sql"),
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

    fn space_styles(conn: &Connection) -> Vec<(String, String, String)> {
        let mut query = conn
            .prepare("SELECT project_id, status_id, style FROM space_status_style ORDER BY 1, 2")
            .unwrap();
        query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn app_styles(conn: &Connection) -> Vec<(String, String)> {
        let mut query = conn
            .prepare("SELECT status_id, style FROM status_style ORDER BY 1")
            .unwrap();
        query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn triples(rows: &[(&str, &str, &str)]) -> Vec<(String, String, String)> {
        rows.iter()
            .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
            .collect()
    }

    fn before_app_wide_styles() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..25]).unwrap();
        conn
    }

    #[test]
    fn spaces_agreeing_on_a_status_style_make_it_app_wide() {
        let mut conn = before_app_wide_styles();
        conn.execute_batch(
            "INSERT INTO space (project_id) VALUES ('10001'), ('10002');
             INSERT INTO space_status_style (project_id, status_id, style) VALUES
               ('10001', 's1', 'blocked'), ('10002', 's1', 'blocked'), ('10001', 's2', 'done');",
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        assert_eq!(
            app_styles(&conn),
            [("s1", "blocked"), ("s2", "done")].map(|(a, b)| (a.to_string(), b.to_string()))
        );
        assert_eq!(space_styles(&conn), triples(&[]));
    }

    #[test]
    fn spaces_disagreeing_on_a_status_style_keep_their_own_and_set_none_app_wide() {
        let mut conn = before_app_wide_styles();
        conn.execute_batch(
            "INSERT INTO space (project_id) VALUES ('10001'), ('10002');
             INSERT INTO space_status_style (project_id, status_id, style) VALUES
               ('10001', 's1', 'blocked'), ('10002', 's1', 'review'),
               ('10001', 's2', 'done'), ('10002', 's2', 'done');",
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        assert_eq!(app_styles(&conn), [("s2".to_string(), "done".to_string())]);
        assert_eq!(
            space_styles(&conn),
            triples(&[("10001", "s1", "blocked"), ("10002", "s1", "review")])
        );
    }

    #[test]
    fn app_wide_status_styles_become_every_spaces_override_when_space_settings_arrive() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..6]).unwrap();
        conn.execute_batch(
            "INSERT INTO space (project_id) VALUES ('10001'), ('10002');
             INSERT INTO status_style (status_id, style) VALUES ('s1', 'blocked'), ('s2', 'done');",
        )
        .unwrap();

        migrate(&mut conn, "local", &LOCAL[..7]).unwrap();

        let rows = space_styles(&conn);
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
    fn spaces_from_before_the_accent_start_without_one() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..21]).unwrap();
        conn.execute(
            "INSERT INTO space (project_id) VALUES ('10001'), ('10002')",
            [],
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let mut query = conn
            .prepare("SELECT project_id, accent FROM space ORDER BY 1")
            .unwrap();
        let rows: Vec<(String, Option<String>)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows,
            [("10001".to_string(), None), ("10002".to_string(), None)]
        );
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
    fn views_from_before_the_saved_person_chip_start_without_one() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..19]).unwrap();
        conn.execute_batch(
            "INSERT INTO view (list_id, name, layout, group_by, sort_by, sort_order)
             VALUES (1, 'Board', 'board', 'status', 'updated', 0),
                    (1, 'List', 'list', 'status', 'updated', 0)",
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let mut query = conn
            .prepare("SELECT name, person FROM view ORDER BY id")
            .unwrap();
        let rows: Vec<(String, Option<String>)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for name in ["Board", "List"] {
            assert!(rows.contains(&(name.to_string(), None)), "{name}");
        }
        assert!(rows.iter().all(|(_, person)| person.is_none()), "{rows:?}");
    }

    #[test]
    fn views_from_before_their_own_me_rules_start_without_any() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..22]).unwrap();
        conn.execute_batch(
            "INSERT INTO view (list_id, name, layout, group_by, sort_by, sort_order)
             VALUES (1, 'Board', 'board', 'status', 'updated', 0),
                    (1, 'List', 'list', 'status', 'updated', 0)",
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let mut query = conn
            .prepare("SELECT name, me_rules FROM view ORDER BY id")
            .unwrap();
        let rows: Vec<(String, Option<String>)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for name in ["Board", "List"] {
            assert!(rows.contains(&(name.to_string(), None)), "{name}");
        }
        assert!(rows.iter().all(|(_, rules)| rules.is_none()), "{rows:?}");
    }

    #[test]
    fn people_switched_off_before_the_member_table_stay_switched_off_after_it() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..16]).unwrap();
        conn.execute_batch(
            "INSERT INTO space_member_off (project_id, account_id)
             VALUES ('10001', 'acc-bea'), ('10002', 'acc-dan')",
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let mut query = conn
            .prepare("SELECT project_id, account_id, active FROM space_member ORDER BY 1, 2")
            .unwrap();
        let rows: Vec<(String, String, bool)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows,
            [
                ("10001".to_string(), "acc-bea".to_string(), false),
                ("10002".to_string(), "acc-dan".to_string(), false),
            ]
        );
        assert!(!tables(&conn).contains(&"space_member_off".to_string()));
    }

    #[test]
    fn dropping_the_unread_list_removes_its_views_and_pins_and_nothing_else() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..17]).unwrap();
        let id_of = |conn: &Connection, kind: &str| -> i64 {
            conn.query_row("SELECT id FROM filter_list WHERE kind = ?1", [kind], |r| {
                r.get(0)
            })
            .unwrap()
        };
        let view_of = |conn: &Connection, list: i64| -> i64 {
            conn.query_row("SELECT id FROM view WHERE list_id = ?1", [list], |r| {
                r.get(0)
            })
            .unwrap()
        };
        let (unread, pinned) = (id_of(&conn, "recent"), id_of(&conn, "pinned"));
        let (unread_view, pinned_view) = (view_of(&conn, unread), view_of(&conn, pinned));
        for (kind, target) in [
            ("filter_list", unread.to_string()),
            ("filter_list", pinned.to_string()),
            ("view", unread_view.to_string()),
            ("view", pinned_view.to_string()),
            ("issue", "NW-1".to_string()),
        ] {
            conn.execute(
                "INSERT INTO pin (target_kind, target_id, created_at) \
                 VALUES (?1, ?2, '2026-01-05T09:00:00Z')",
                [kind, &target],
            )
            .unwrap();
        }

        migrate(&mut conn, "local", LOCAL).unwrap();

        let strings = |sql: &str| -> Vec<String> {
            conn.prepare(sql)
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        assert_eq!(
            strings("SELECT kind FROM filter_list ORDER BY id"),
            ["everything", "pinned", "viewed"]
        );
        assert_eq!(
            strings("SELECT CAST(id AS TEXT) FROM view ORDER BY id").len(),
            3
        );
        assert!(!strings("SELECT CAST(id AS TEXT) FROM view").contains(&unread_view.to_string()));
        assert_eq!(
            strings("SELECT target_kind || ':' || target_id FROM pin ORDER BY id"),
            [
                format!("filter_list:{pinned}"),
                format!("view:{pinned_view}"),
                "issue:NW-1".to_string(),
            ]
        );
    }

    #[test]
    fn the_write_log_migration_adds_an_empty_log_and_keeps_the_rest() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..20]).unwrap();
        conn.execute(
            "INSERT INTO pin (target_kind, target_id, created_at) \
             VALUES ('issue', 'NW-1', '2026-01-05T09:00:00Z')",
            [],
        )
        .unwrap();
        assert!(!tables(&conn).contains(&"write_log".to_string()));

        migrate(&mut conn, "local", LOCAL).unwrap();

        assert!(tables(&conn).contains(&"write_log".to_string()));
        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(count("write_log"), 0);
        assert_eq!(count("pin"), 1);
        conn.execute(
            "INSERT INTO write_log (at, issue_id, change) \
             VALUES ('2026-10-28T12:00:00.000Z', '20001', 'Status: Done')",
            [],
        )
        .unwrap();
        let error: Option<String> = conn
            .query_row("SELECT error FROM write_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(error, None);
    }

    #[test]
    fn the_recently_viewed_migration_adds_one_node_with_a_day_window_and_its_default_view() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..24]).unwrap();
        let before: i64 = conn
            .query_row("SELECT COUNT(*) FROM filter_list", [], |r| r.get(0))
            .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let nodes: Vec<(String, String, i64, Option<i64>, String)> = conn
            .prepare(
                "SELECT kind, name, position, parent_id, filter FROM filter_list WHERE kind = 'viewed'",
            )
            .unwrap()
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(nodes.len(), 1);
        let (kind, name, position, parent, filter) = nodes[0].clone();
        assert_eq!(
            (kind.as_str(), name.as_str(), position, parent),
            ("viewed", "Recently viewed", -1, None)
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&filter).unwrap(),
            serde_json::json!({
                "conditions": [{ "field": "lastView", "op": "withinLastHours", "value": 24 }]
            })
        );
        let after: i64 = conn
            .query_row("SELECT COUNT(*) FROM filter_list", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after, before + 1);
        let view: (String, String, String, bool, String, String, i64, String) = conn
            .query_row(
                "SELECT name, layout, columns, is_default, group_by, sort_by, sort_order, subtasks
                 FROM view WHERE list_id = (SELECT id FROM filter_list WHERE kind = 'viewed')",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            view,
            (
                "All issues".to_string(),
                "list".to_string(),
                r#"["lastView","assignee","status","priority"]"#.to_string(),
                true,
                "none".to_string(),
                "lastView".to_string(),
                -1,
                "hidden".to_string(),
            )
        );
    }

    #[test]
    fn existing_bookmarks_are_numbered_newest_first_and_start_without_a_title() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, "local", &LOCAL[..18]).unwrap();
        conn.execute_batch(
            "INSERT INTO pin (target_kind, target_id, created_at) VALUES
               ('issue', 'NW-1', '2026-01-05T09:00:00Z'),
               ('issue', 'NW-2', '2026-01-05T09:00:00Z'),
               ('issue', 'NW-3', '2026-01-04T09:00:00Z'),
               ('issue', 'NW-4', '2026-01-06T09:00:00Z')",
        )
        .unwrap();

        migrate(&mut conn, "local", LOCAL).unwrap();

        let mut query = conn
            .prepare("SELECT target_id, position, title FROM pin ORDER BY position")
            .unwrap();
        let rows: Vec<(String, i64, Option<String>)> = query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows,
            [
                ("NW-4".to_string(), 0, None),
                ("NW-2".to_string(), 1, None),
                ("NW-1".to_string(), 2, None),
                ("NW-3".to_string(), 3, None),
            ]
        );
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
