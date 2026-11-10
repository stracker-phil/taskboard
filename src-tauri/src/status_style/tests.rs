use super::*;
use crate::db::Db;

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
           status_category, created_at, updated_at, synced_at, gone)
         VALUES ('{id}', 'K-{id}', '{project}', 'Summary {id}', 'Task', '{status}', '{category}',
           '2026-10-20T09:00:00.000Z', '2026-10-20T09:00:00.000Z',
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
