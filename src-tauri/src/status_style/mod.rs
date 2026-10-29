//! One status language: every Jira status shows in one of six styles (status types), detected from its category and
//! name unless the owner picked one (`status_style`).

pub mod commands;

use std::collections::HashMap;

use crate::filter::TRACKED_ISSUES;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{Db, LocalPart};
use crate::error::{AppError, AppResult};
use crate::views::model::{from_text, to_text};

/// Stored as its name; the `--status-{style}-*` tokens draw it. Ordered as the list's status groups.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Style {
    Progress,
    Review,
    Blocked,
    Todo,
    Backlog,
    Done,
}

impl Style {
    /// The status type's name, as the Status type group and column show it.
    pub fn name(self) -> &'static str {
        match self {
            Style::Backlog => "Backlog",
            Style::Todo => "To do",
            Style::Progress => "In progress",
            Style::Review => "In review",
            Style::Blocked => "Blocked",
            Style::Done => "Done",
        }
    }
}

/// Jira's category decides To do and Done; the name tells backlog apart from to do, and review and blocked
/// apart from in progress.
pub fn detect(category: &str, name: &str) -> Style {
    let name = name.to_lowercase();
    match category {
        "new" if name.contains("backlog") => Style::Backlog,
        "new" => Style::Todo,
        "done" => Style::Done,
        _ if name.contains("review") => Style::Review,
        _ if name.contains("block") => Style::Blocked,
        _ => Style::Progress,
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StatusStyleRow {
    pub id: String,
    pub name: String,
    /// Jira's status category: `new`, `indeterminate` or `done`.
    pub category: String,
    pub detected: Style,
    /// The owner's choice in this Space; none = Automatic.
    pub chosen: Option<Style>,
    /// The Space's issues in this status.
    pub issues: i64,
}

impl StatusStyleRow {
    pub fn style(&self) -> Style {
        self.chosen.unwrap_or(self.detected)
    }
}

/// The statuses a Space's issues are in (the rest of the site's workflows don't matter here), by Jira category
/// (to do, in progress, done), then name.
pub fn status_rows(conn: &Connection, project_id: &str) -> AppResult<Vec<StatusStyleRow>> {
    let mut query = conn.prepare(&format!(
        "SELECT s.id, s.name, s.category, o.style, used.issues
         FROM cache.status s
         JOIN (SELECT i.status_id, COUNT(*) AS issues FROM cache.issue i
               WHERE {TRACKED_ISSUES} AND i.project_id = ?1
               GROUP BY i.status_id) used ON used.status_id = s.id
         LEFT JOIN space_status_style o ON o.status_id = s.id AND o.project_id = ?1
         ORDER BY CASE s.category WHEN 'new' THEN 0 WHEN 'indeterminate' THEN 1 ELSE 2 END,
                  s.name COLLATE NOCASE, s.id"
    ))?;
    let rows = query.query_map([project_id], |row| {
        let name: String = row.get(1)?;
        let category: String = row.get(2)?;
        let chosen: Option<String> = row.get(3)?;
        Ok(StatusStyleRow {
            id: row.get(0)?,
            detected: detect(&category, &name),
            chosen: chosen.as_deref().and_then(from_text),
            issues: row.get(4)?,
            name,
            category,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The owner's style choices of every Space; a status without one shows its detected style.
pub struct Styles(HashMap<(String, String), Style>);

impl Styles {
    pub fn load(conn: &Connection) -> AppResult<Self> {
        let mut query =
            conn.prepare("SELECT project_id, status_id, style FROM space_status_style")?;
        let rows = query.query_map([], |row| {
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut chosen = HashMap::new();
        for row in rows {
            let (key, style) = row?;
            if let Some(style) = from_text(&style) {
                chosen.insert(key, style);
            }
        }
        Ok(Self(chosen))
    }

    /// A status's style in a Space: the owner's choice there, else detected from its category and name.
    pub fn of(&self, project_id: &str, status_id: &str, category: &str, name: &str) -> Style {
        self.0
            .get(&(project_id.to_string(), status_id.to_string()))
            .copied()
            .unwrap_or_else(|| detect(category, name))
    }
}

pub async fn status_styles_list(db: &Db, project_id: String) -> AppResult<Vec<StatusStyleRow>> {
    db.read(move |conn| status_rows(conn, &project_id)).await
}

/// `None` goes back to the detected style.
pub async fn status_style_set(
    db: &Db,
    project_id: String,
    status_id: String,
    style: Option<Style>,
) -> AppResult<Vec<StatusStyleRow>> {
    db.write(move |tx, changes| {
        let known: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM cache.status WHERE id = ?1)",
            [&status_id],
            |row| row.get(0),
        )?;
        if !known {
            return Err(AppError::NotFound(format!(
                "Status {status_id} doesn't exist"
            )));
        }
        match style {
            Some(style) => tx.execute(
                "INSERT INTO space_status_style (project_id, status_id, style) VALUES (?1, ?2, ?3)
                 ON CONFLICT (project_id, status_id) DO UPDATE SET style = excluded.style",
                params![project_id, status_id, to_text(style)],
            )?,
            None => tx.execute(
                "DELETE FROM space_status_style WHERE project_id = ?1 AND status_id = ?2",
                [&project_id, &status_id],
            )?,
        };
        changes.local_changed(LocalPart::Styles);
        status_rows(tx, &project_id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_follows_the_category_and_then_the_name() {
        for (category, name, expected) in [
            ("new", "To Do", Style::Todo),
            ("new", "Blocked", Style::Todo),
            ("new", "Backlog", Style::Backlog),
            ("new", "Product BACKLOG", Style::Backlog),
            ("indeterminate", "Backlog", Style::Progress),
            ("done", "Backlog", Style::Done),
            ("done", "Done", Style::Done),
            ("done", "Done review", Style::Done),
            ("indeterminate", "In Review", Style::Review),
            ("indeterminate", "Code review", Style::Review),
            ("indeterminate", "Blocked", Style::Blocked),
            ("indeterminate", "Blocker found", Style::Blocked),
            ("indeterminate", "In Progress", Style::Progress),
            ("indeterminate", "Testing", Style::Progress),
        ] {
            assert_eq!(detect(category, name), expected, "{category} / {name}");
        }
    }

    #[test]
    fn styles_rank_in_the_lists_group_order() {
        let mut styles = [
            Style::Done,
            Style::Backlog,
            Style::Todo,
            Style::Blocked,
            Style::Review,
            Style::Progress,
        ];
        styles.sort();
        assert_eq!(
            styles,
            [
                Style::Progress,
                Style::Review,
                Style::Blocked,
                Style::Todo,
                Style::Backlog,
                Style::Done
            ]
        );
    }

    #[test]
    fn each_style_has_a_readable_name() {
        for (style, expected) in [
            (Style::Backlog, "Backlog"),
            (Style::Todo, "To do"),
            (Style::Progress, "In progress"),
            (Style::Review, "In review"),
            (Style::Blocked, "Blocked"),
            (Style::Done, "Done"),
        ] {
            assert_eq!(style.name(), expected, "{style:?}");
        }
    }

    #[test]
    fn a_chosen_style_wins_over_the_detected_one() {
        let row = |chosen| StatusStyleRow {
            id: "10001".into(),
            name: "In Review".into(),
            category: "indeterminate".into(),
            detected: Style::Review,
            chosen,
            issues: 0,
        };
        assert_eq!(row(Some(Style::Blocked)).style(), Style::Blocked);
        assert_eq!(row(None).style(), Style::Review);
    }

    fn issue(id: u32, project: &str, status: &str, category: &str, gone: bool) -> String {
        format!(
            "INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id,
               status_category, created_at, updated_at, raw, synced_at, gone)
             VALUES ('{id}', 'K-{id}', '{project}', 'Summary {id}', 'Task', '{status}', '{category}',
               '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z', '{{}}',
               '2026-10-20T09:00:00.000Z', {});",
            gone as i32
        )
    }

    /// Website relaunch (`10001`) is tracked, Northwind support (`10002`) is not.
    async fn lab() -> (tempfile::TempDir, Db) {
        let dir = crate::test_support::temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        let issues: String = [
            issue(1, "10001", "s1", "new", false),
            issue(2, "10001", "s1", "new", false),
            issue(3, "10001", "s2", "indeterminate", false),
            issue(4, "10001", "s3", "indeterminate", false),
            issue(5, "10001", "s3", "indeterminate", false),
            issue(6, "10001", "s3", "indeterminate", false),
            issue(7, "10001", "s4", "done", false),
            issue(8, "10001", "s4", "done", true),
            issue(9, "10001", "s5", "new", false),
            issue(10, "10001", "s8", "indeterminate", true),
            issue(11, "10002", "s6", "done", false),
            issue(12, "10002", "s1", "new", false),
        ]
        .concat();
        let sql = format!(
            "INSERT INTO cache.space (id, key, name, raw) VALUES
               ('10001', 'NW', 'Website relaunch', '{{}}'), ('10002', 'NWS', 'Northwind support', '{{}}');
             INSERT INTO cache.status (id, name, category) VALUES
               ('s4', 'Done', 'done'), ('s2', 'Testing', 'indeterminate'), ('s1', 'To Do', 'new'),
               ('s3', 'in review', 'indeterminate'), ('s5', 'Backlog', 'new'), ('s6', 'Archived', 'done'),
               ('s7', 'Unused', 'new'), ('s8', 'Withdrawn', 'indeterminate');
             {issues}"
        );
        db.write(move |tx, _| {
            tx.execute_batch(&sql)?;
            Ok(())
        })
        .await
        .unwrap();
        crate::connection::track_project(&db, "10001".into(), true)
            .await
            .unwrap();
        (dir, db)
    }

    fn named_counts(rows: &[StatusStyleRow]) -> Vec<(&str, i64)> {
        rows.iter().map(|r| (r.name.as_str(), r.issues)).collect()
    }

    #[tokio::test]
    async fn a_status_only_untracked_projects_use_is_not_listed() {
        let (_dir, db) = lab().await;

        let rows = status_styles_list(&db, "10001".into()).await.unwrap();

        assert!(rows.iter().all(|r| r.id != "s6"), "{rows:?}");
    }

    #[tokio::test]
    async fn a_status_without_issues_is_not_listed() {
        let (_dir, db) = lab().await;

        let rows = status_styles_list(&db, "10001".into()).await.unwrap();

        assert!(rows.iter().all(|r| r.id != "s7"), "{rows:?}");
    }

    #[tokio::test]
    async fn a_status_only_gone_issues_use_is_not_listed_and_gone_issues_do_not_count() {
        let (_dir, db) = lab().await;

        let rows = status_styles_list(&db, "10001".into()).await.unwrap();

        assert!(rows.iter().all(|r| r.id != "s8"), "{rows:?}");
        let done = rows.iter().find(|r| r.id == "s4").unwrap();
        assert_eq!(done.issues, 1);
    }

    #[tokio::test]
    async fn the_count_is_the_tracked_issues_in_each_status() {
        let (_dir, db) = lab().await;

        let rows = status_styles_list(&db, "10001".into()).await.unwrap();

        let by_id = |id: &str| rows.iter().find(|r| r.id == id).unwrap().issues;
        assert_eq!(
            [
                by_id("s1"),
                by_id("s2"),
                by_id("s3"),
                by_id("s4"),
                by_id("s5")
            ],
            [2, 1, 3, 1, 1]
        );
    }

    #[tokio::test]
    async fn statuses_are_ordered_by_category_then_name_ignoring_case() {
        let (_dir, db) = lab().await;

        let rows = status_styles_list(&db, "10001".into()).await.unwrap();

        assert_eq!(
            named_counts(&rows),
            [
                ("Backlog", 1),
                ("To Do", 2),
                ("in review", 3),
                ("Testing", 1),
                ("Done", 1),
            ]
        );
    }

    #[tokio::test]
    async fn setting_a_style_on_a_listed_status_returns_the_list_with_the_choice() {
        let (_dir, db) = lab().await;

        let rows = status_style_set(&db, "10001".into(), "s3".into(), Some(Style::Blocked))
            .await
            .unwrap();

        assert_eq!(
            named_counts(&rows),
            [
                ("Backlog", 1),
                ("To Do", 2),
                ("in review", 3),
                ("Testing", 1),
                ("Done", 1),
            ]
        );
        let chosen: Vec<(&str, Option<Style>)> = rows
            .iter()
            .filter(|r| r.chosen.is_some())
            .map(|r| (r.id.as_str(), r.chosen))
            .collect();
        assert_eq!(chosen, [("s3", Some(Style::Blocked))]);
        let reset = status_style_set(&db, "10001".into(), "s3".into(), None)
            .await
            .unwrap();
        assert!(reset.iter().all(|r| r.chosen.is_none()));
    }

    #[tokio::test]
    async fn the_list_holds_the_statuses_of_that_spaces_issues_only() {
        let (_dir, db) = lab().await;
        crate::connection::track_project(&db, "10002".into(), true)
            .await
            .unwrap();

        let rows = status_styles_list(&db, "10002".into()).await.unwrap();

        assert_eq!(named_counts(&rows), [("To Do", 1), ("Archived", 1)]);
    }

    #[tokio::test]
    async fn a_style_chosen_in_one_space_does_not_reach_the_same_status_in_another() {
        let (_dir, db) = lab().await;
        crate::connection::track_project(&db, "10002".into(), true)
            .await
            .unwrap();

        let in_a = status_style_set(&db, "10001".into(), "s1".into(), Some(Style::Blocked))
            .await
            .unwrap();
        let in_b = status_styles_list(&db, "10002".into()).await.unwrap();
        let styles = db.read(Styles::load).await.unwrap();

        let chosen = |rows: &[StatusStyleRow]| rows.iter().find(|r| r.id == "s1").unwrap().chosen;
        assert_eq!(chosen(&in_a), Some(Style::Blocked));
        assert_eq!(chosen(&in_b), None);
        assert_eq!(styles.of("10001", "s1", "new", "To Do"), Style::Blocked);
        assert_eq!(styles.of("10002", "s1", "new", "To Do"), Style::Todo);
        assert_eq!(
            styles.of("10001", "s2", "indeterminate", "Testing"),
            Style::Progress
        );
    }

    #[tokio::test]
    async fn the_same_status_can_be_styled_differently_in_two_spaces_and_reset_on_its_own() {
        let (_dir, db) = lab().await;
        crate::connection::track_project(&db, "10002".into(), true)
            .await
            .unwrap();
        status_style_set(&db, "10001".into(), "s1".into(), Some(Style::Blocked))
            .await
            .unwrap();
        status_style_set(&db, "10002".into(), "s1".into(), Some(Style::Review))
            .await
            .unwrap();

        status_style_set(&db, "10001".into(), "s1".into(), None)
            .await
            .unwrap();

        let styles = db.read(Styles::load).await.unwrap();
        assert_eq!(styles.of("10001", "s1", "new", "To Do"), Style::Todo);
        assert_eq!(styles.of("10002", "s1", "new", "To Do"), Style::Review);
    }

    #[tokio::test]
    async fn styling_an_unknown_status_is_not_found() {
        let (_dir, db) = lab().await;

        let err = status_style_set(&db, "10001".into(), "nope".into(), Some(Style::Done))
            .await
            .unwrap_err();

        assert_eq!(err.kind(), crate::error::ErrorKind::NotFound);
    }
}
