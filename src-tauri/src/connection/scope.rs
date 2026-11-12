//! The sync scope: one JQL query that decides which issues live in the cache.

use serde::Serialize;
use ts_rs::TS;

use super::projects::tracked_keys;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::jira::{JiraClient, jql};
use crate::store::settings::{SettingsStore, SyncSettings};
use crate::sync::store::forget_types;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ScopeView {
    /// Empty while nothing is tracked and there is no override.
    pub jql: String,
    pub is_default: bool,
}

/// The override, or the default over the tracked projects.
pub fn effective_scope(sync: &SyncSettings, tracked_keys: &[String]) -> ScopeView {
    match &sync.scope_override {
        Some(jql) => ScopeView {
            jql: jql.clone(),
            is_default: false,
        },
        None => ScopeView {
            jql: jql::default_scope(tracked_keys, sync.keep_done_days).unwrap_or_default(),
            is_default: true,
        },
    }
}

/// The scope as the Sync page shows it.
pub async fn scope_view(db: &Db, sync: &SyncSettings) -> AppResult<ScopeView> {
    Ok(effective_scope(sync, &tracked_keys(db).await?))
}

/// Saves everything but the scope, which changes only through `save_scope`.
pub async fn save_intervals(
    settings: &SettingsStore,
    sync: SyncSettings,
) -> AppResult<SyncSettings> {
    sync.validate()?;
    let sync = sync.clamped();
    let saved = settings
        .edit(move |s| {
            s.sync = SyncSettings {
                scope_override: s.sync.scope_override.take(),
                ..sync
            }
        })
        .await?;
    Ok(saved.sync)
}

/// The query a sync runs: the scope without the excluded issue types; empty while nothing is tracked.
pub fn sync_query(sync: &SyncSettings, tracked_keys: &[String]) -> String {
    if tracked_keys.is_empty() {
        return String::new();
    }
    jql::without_types(
        &effective_scope(sync, tracked_keys).jql,
        &sync.excluded_types,
    )
}

/// After the excluded types changed: removes the cached issues of a newly excluded type at once. Returns whether
/// a type came back, which only a full sync fetches again.
pub async fn apply_excluded_types(db: &Db, before: &[String], after: &[String]) -> AppResult<bool> {
    let left_out: Vec<String> = after
        .iter()
        .filter(|t| !before.contains(t))
        .cloned()
        .collect();
    if !left_out.is_empty() {
        db.write(move |tx, changes| {
            for id in forget_types(tx, &left_out)? {
                changes.issue_changed(id);
            }
            Ok(())
        })
        .await?;
    }
    Ok(before.iter().any(|t| !after.contains(t)))
}

/// The cached issues' types and `excluded`, once each, by name.
pub fn issue_types(conn: &rusqlite::Connection, excluded: &[String]) -> AppResult<Vec<String>> {
    let mut query = conn.prepare(
        "SELECT issue_type FROM cache.issue
         UNION SELECT value FROM json_each(?1)
         ORDER BY 1 COLLATE NOCASE",
    )?;
    let types = query.query_map([serde_json::to_string(excluded)?], |row| row.get(0))?;
    Ok(types.collect::<Result<_, _>>()?)
}

/// Jira's strict check; its messages come back as `InvalidJql`.
pub async fn validate_scope(client: &JiraClient, query: &str) -> AppResult<()> {
    if query.trim().is_empty() {
        return Err(AppError::InvalidJql(vec!["Enter a JQL query".into()]));
    }
    let problems = client.check_jql(query).await?;
    if problems.is_empty() {
        Ok(())
    } else {
        Err(AppError::InvalidJql(problems))
    }
}

/// Saves the owner's scope after Jira accepted it; `None` goes back to the default.
pub async fn save_scope(
    settings: &SettingsStore,
    client: Option<&JiraClient>,
    query: Option<String>,
) -> AppResult<SyncSettings> {
    let query = query.map(|q| q.trim().to_string());
    if let Some(query) = &query {
        let client = client.ok_or_else(|| AppError::NotConnected("Connect Jira first".into()))?;
        validate_scope(client, query).await?;
    }
    let saved = settings
        .edit(move |s| s.sync.scope_override = query)
        .await?;
    Ok(saved.sync)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn without_an_override_the_scope_is_the_default_over_the_tracked_projects() {
        let sync = SyncSettings {
            keep_done_days: 30,
            ..SyncSettings::default()
        };
        let tracked = keys(&["NW", "LUM"]);

        assert_eq!(
            effective_scope(&sync, &tracked),
            ScopeView {
                jql: jql::default_scope(&tracked, 30).unwrap(),
                is_default: true,
            }
        );
    }

    #[test]
    fn without_tracked_projects_or_an_override_the_scope_is_empty() {
        assert_eq!(
            effective_scope(&SyncSettings::default(), &[]),
            ScopeView {
                jql: String::new(),
                is_default: true,
            }
        );
    }

    #[test]
    fn an_override_replaces_the_default_whatever_is_tracked() {
        let sync = SyncSettings {
            scope_override: Some("assignee = currentUser()".into()),
            ..SyncSettings::default()
        };

        for tracked in [keys(&[]), keys(&["NW"])] {
            assert_eq!(
                effective_scope(&sync, &tracked),
                ScopeView {
                    jql: "assignee = currentUser()".into(),
                    is_default: false,
                },
                "{tracked:?}"
            );
        }
    }

    #[tokio::test]
    async fn saving_the_intervals_keeps_the_scope_override_and_saves_clear_on_quit() {
        let dir = crate::test_support::temp_config_dir();
        let settings = SettingsStore::open(dir.path());
        settings
            .edit(|s| s.sync.scope_override = Some("project = NW".into()))
            .await
            .unwrap();

        let saved = save_intervals(
            &settings,
            SyncSettings {
                scope_override: None,
                focused_minutes: 10,
                background_minutes: 30,
                ..SyncSettings::default()
            },
        )
        .await
        .unwrap();

        let expected = SyncSettings {
            scope_override: Some("project = NW".into()),
            focused_minutes: 10,
            background_minutes: 30,
            ..SyncSettings::default()
        };
        assert_eq!(saved, expected);
        assert_eq!(SettingsStore::open(dir.path()).get().sync, expected);
    }

    fn excluding(names: &[&str]) -> SyncSettings {
        SyncSettings {
            excluded_types: keys(names),
            ..SyncSettings::default()
        }
    }

    #[test]
    fn the_sync_query_is_empty_while_nothing_is_tracked() {
        assert_eq!(sync_query(&excluding(&["Test"]), &[]), "");
    }

    #[test]
    fn the_sync_query_is_the_default_scope_without_excluded_types() {
        let tracked = keys(&["NW"]);
        let default = jql::default_scope(&tracked, 90).unwrap();

        assert_eq!(sync_query(&SyncSettings::default(), &tracked), default);
        assert_eq!(
            sync_query(&excluding(&["Test", "Test Execution"]), &tracked),
            format!(r#"({default}) AND issuetype NOT IN ("Test", "Test Execution")"#)
        );
    }

    #[test]
    fn the_sync_query_leaves_excluded_types_out_of_an_override_too() {
        let sync = SyncSettings {
            scope_override: Some("assignee = currentUser()".into()),
            excluded_types: keys(&["Test"]),
            ..SyncSettings::default()
        };

        assert_eq!(
            sync_query(&sync, &keys(&["NW"])),
            r#"(assignee = currentUser()) AND issuetype NOT IN ("Test")"#
        );
    }

    #[tokio::test]
    async fn saving_the_intervals_stores_the_tidied_excluded_types() {
        let dir = crate::test_support::temp_config_dir();
        let settings = SettingsStore::open(dir.path());

        let saved = save_intervals(&settings, excluding(&[" Test ", "Bug", "", "Bug"]))
            .await
            .unwrap();

        assert_eq!(saved.excluded_types, keys(&["Bug", "Test"]));
        assert_eq!(
            SettingsStore::open(dir.path()).get().sync.excluded_types,
            keys(&["Bug", "Test"])
        );
    }

    async fn db_with_types(types: &[(&str, &str)]) -> (tempfile::TempDir, Db) {
        let dir = crate::test_support::temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        let mut sql = String::from(
            "INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Website relaunch', '{}');",
        );
        for (id, issue_type) in types {
            sql.push_str(&format!(
                "INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
                                         created_at, updated_at, synced_at, gone)
                 VALUES ('{id}', 'NW-{id}', '10001', 'Issue {id}', '{issue_type}', 's1', 'new',
                         '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 0);
                 INSERT INTO cache.issue_fts (rowid, key, summary, description, comments)
                 VALUES ({id}, 'NW-{id}', 'Issue {id}', '', '');
                 INSERT INTO cache.comment (id, issue_id, created_at, updated_at)
                 VALUES ('c{id}', '{id}', '2026-01-02T00:00:00Z', '2026-01-02T00:00:00Z');
                 INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
                 VALUES ('{id}', 'priority', 'High', 'Low', '2026-01-02T00:00:00Z');
                 INSERT INTO issue_seen (issue_id, seen_at) VALUES ('{id}', '2026-01-03T00:00:00Z');
                 INSERT INTO issue_activity (issue_id, viewed_at, interacted_at)
                 VALUES ('{id}', NULL, '2026-01-06T00:00:00.000Z');"
            ));
        }
        db.write(move |tx, _| {
            tx.execute_batch(&sql)?;
            Ok(())
        })
        .await
        .unwrap();
        (dir, db)
    }

    async fn count(db: &Db, sql: &'static str) -> i64 {
        db.read(move |conn| Ok(conn.query_row(sql, [], |row| row.get(0))?))
            .await
            .unwrap()
    }

    async fn types_listed(db: &Db, excluded: &[&str]) -> Vec<String> {
        let excluded = keys(excluded);
        db.read(move |conn| issue_types(conn, &excluded))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn the_issue_types_are_the_cached_ones_and_the_excluded_ones_once_each_sorted_by_name() {
        let (_dir, db) =
            db_with_types(&[("1", "Story"), ("2", "bug"), ("3", "Story"), ("4", "Epic")]).await;

        assert_eq!(
            types_listed(&db, &["Test", "Story"]).await,
            keys(&["bug", "Epic", "Story", "Test"])
        );
        assert_eq!(
            types_listed(&db, &[]).await,
            keys(&["bug", "Epic", "Story"])
        );
    }

    #[tokio::test]
    async fn without_cached_issues_the_issue_types_are_the_excluded_ones() {
        let (_dir, db) = db_with_types(&[]).await;

        assert_eq!(types_listed(&db, &["Test"]).await, keys(&["Test"]));
        assert_eq!(types_listed(&db, &[]).await, Vec::<String>::new());
    }

    #[tokio::test]
    async fn excluding_a_type_deletes_its_cached_issues_and_what_belongs_to_them() {
        let (_dir, db) =
            db_with_types(&[("1", "Test"), ("2", "Story"), ("3", "Test Execution")]).await;

        let came_back = apply_excluded_types(&db, &keys(&[]), &keys(&["Test"]))
            .await
            .unwrap();

        assert!(!came_back);
        for sql in [
            "SELECT COUNT(*) FROM cache.issue WHERE id = '1'",
            "SELECT COUNT(*) FROM cache.issue_fts WHERE rowid = 1",
            "SELECT COUNT(*) FROM cache.comment WHERE issue_id = '1'",
            "SELECT COUNT(*) FROM cache.issue_change WHERE issue_id = '1'",
            "SELECT COUNT(*) FROM issue_seen WHERE issue_id = '1'",
            "SELECT COUNT(*) FROM issue_activity WHERE issue_id = '1'",
        ] {
            assert_eq!(count(&db, sql).await, 0, "{sql}");
        }
        for sql in [
            "SELECT COUNT(*) FROM cache.issue WHERE id IN ('2', '3')",
            "SELECT COUNT(*) FROM cache.comment WHERE issue_id IN ('2', '3')",
            "SELECT COUNT(*) FROM issue_seen WHERE issue_id IN ('2', '3')",
            "SELECT COUNT(*) FROM issue_activity WHERE issue_id IN ('2', '3')",
        ] {
            assert_eq!(count(&db, sql).await, 2, "{sql}");
        }
    }

    #[tokio::test]
    async fn only_a_type_that_came_back_asks_for_a_full_sync() {
        let (_dir, db) = db_with_types(&[("1", "Test"), ("2", "Story")]).await;

        for (before, after, expected) in [
            (vec![], vec![], false),
            (vec!["Test"], vec!["Test"], false),
            (vec!["Test"], vec![], true),
            (vec!["Test", "Bug"], vec!["Bug"], true),
            (vec!["Bug"], vec!["Bug", "Epic"], false),
            (vec!["Bug"], vec!["Epic"], true),
        ] {
            assert_eq!(
                apply_excluded_types(&db, &keys(&before), &keys(&after))
                    .await
                    .unwrap(),
                expected,
                "{before:?} -> {after:?}"
            );
        }
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.issue").await, 2);
    }

    #[tokio::test]
    async fn including_a_type_again_keeps_the_cached_issues() {
        let (_dir, db) = db_with_types(&[("1", "Test"), ("2", "Story")]).await;

        let came_back = apply_excluded_types(&db, &keys(&["Test"]), &keys(&[]))
            .await
            .unwrap();

        assert!(came_back);
        assert_eq!(count(&db, "SELECT COUNT(*) FROM cache.issue").await, 2);
    }

    #[test]
    fn the_scope_view_uses_camel_case_names_for_the_page() {
        let view = ScopeView {
            jql: "project = NW".into(),
            is_default: false,
        };
        assert_eq!(
            serde_json::to_value(view).unwrap(),
            json!({ "jql": "project = NW", "isDefault": false })
        );
    }
}
