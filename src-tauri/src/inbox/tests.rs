use super::*;
use crate::changes::{mark_seen, mark_unread};
use crate::db::Db;
use crate::status_style::Style;
use crate::test_support::temp_config_dir;

const SEEN_LATER: &str = "2026-02-01T00:00:00.000Z";

const SETUP: &str = "
    INSERT INTO cache.space (id, key, name, raw) VALUES
      ('10001', 'NW', 'Northwind', '{}'),
      ('10002', 'OT', 'Off Track', '{}');
    INSERT INTO space (project_id, tracked) VALUES ('10001', 1), ('10002', 0);
    INSERT INTO cache.status (id, name, category) VALUES
      ('1', 'To Do', 'new'), ('2', 'Review', 'indeterminate');
    INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
      created_at, updated_at, synced_at, gone)
    VALUES
      ('20001', 'NW-1', '10001', 'Mount the column', 'Task', '2', 'indeterminate',
        '2026-01-05T09:00:00Z', '2026-01-06T09:00:00Z', '2026-01-06T09:00:00Z', 0),
      ('20002', 'NW-2', '10001', 'Paint the wall', 'Task', '1', 'new',
        '2026-01-05T09:00:00Z', '2026-01-06T10:00:00Z', '2026-01-06T10:00:00Z', 0),
      ('20003', 'NW-3', '10001', 'Hang the door', 'Task', '1', 'new',
        '2026-01-05T09:00:00Z', '2026-01-06T09:30:00Z', '2026-01-06T09:30:00Z', 0),
      ('20004', 'NW-4', '10001', 'Sand the floor', 'Task', '1', 'new',
        '2026-01-05T09:00:00Z', '2026-01-06T09:45:00Z', '2026-01-06T09:45:00Z', 0),
      ('20005', 'NW-5', '10001', 'Fix the gate', 'Task', '1', 'new',
        '2026-01-05T09:00:00Z', '2026-01-06T11:00:00Z', '2026-01-06T11:00:00Z', 1),
      ('20006', 'OT-1', '10002', 'Water the plants', 'Task', '1', 'new',
        '2026-01-05T09:00:00Z', '2026-01-06T12:00:00Z', '2026-01-06T12:00:00Z', 0);
    INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at) VALUES
      ('20001', 'status', '1', '2', '2026-01-06T09:00:00Z'),
      ('20002', 'comment_added', '0', '1', '2026-01-06T10:00:00Z'),
      ('20004', 'status', '2', '1', '2026-01-06T08:00:00Z'),
      ('20005', 'status', '1', '2', '2026-01-06T11:00:00Z'),
      ('20006', 'status', '1', '2', '2026-01-06T12:00:00Z');
    INSERT INTO issue_seen (issue_id, seen_at, unread) VALUES
      ('20003', '2026-01-05T09:00:00Z', 1),
      ('20004', '2026-01-07T00:00:00Z', 0);";

async fn seeded() -> (tempfile::TempDir, Db) {
    let dir = temp_config_dir();
    let db = Db::open(dir.path()).unwrap();
    db.write(|tx, _| Ok(tx.execute_batch(SETUP)?))
        .await
        .unwrap();
    (dir, db)
}

async fn open(db: &Db) -> (Vec<Snapshot>, Inbox) {
    db.read(|conn| {
        let taken = snapshot(conn, None)?;
        let inbox = rows(conn, &taken)?;
        Ok((taken, inbox))
    })
    .await
    .unwrap()
}

async fn rows_of(db: &Db, taken: Vec<Snapshot>) -> Inbox {
    db.read(move |conn| rows(conn, &taken)).await.unwrap()
}

fn keys(inbox: &Inbox) -> Vec<&str> {
    inbox.rows.iter().map(|row| row.key.as_str()).collect()
}

fn ids(taken: &[Snapshot]) -> Vec<&str> {
    taken.iter().map(|snap| snap.id.as_str()).collect()
}

#[tokio::test]
async fn unread_issues_are_listed_newest_change_first() {
    let (_dir, db) = seeded().await;

    let (taken, inbox) = open(&db).await;

    assert_eq!(ids(&taken), ["20002", "20003", "20001"]);
    assert_eq!(keys(&inbox), ["NW-2", "NW-3", "NW-1"]);
    assert_eq!(inbox.unread, 3);
    assert!(inbox.rows.iter().all(|row| row.unread));
}

#[tokio::test]
async fn an_issue_only_marked_unread_is_ordered_by_its_updated_time() {
    let (_dir, db) = seeded().await;

    let (_, inbox) = open(&db).await;

    let marked = inbox.rows.iter().find(|row| row.key == "NW-3").unwrap();
    assert_eq!(marked.changed_at, "2026-01-06T09:30:00Z");
}

#[tokio::test]
async fn read_gone_and_untracked_issues_are_not_listed() {
    let (_dir, db) = seeded().await;

    let (_, inbox) = open(&db).await;

    for hidden in ["NW-4", "NW-5", "OT-1"] {
        assert!(!keys(&inbox).contains(&hidden), "{hidden}");
    }
}

#[tokio::test]
async fn a_row_carries_the_issue_its_status_and_a_note() {
    let (_dir, db) = seeded().await;

    let (_, inbox) = open(&db).await;

    assert_eq!(
        inbox.rows[2],
        InboxRow {
            id: "20001".into(),
            key: "NW-1".into(),
            summary: "Mount the column".into(),
            project_id: "10001".into(),
            style: Style::Review,
            status_name: "Review".into(),
            note: "Moved to Review.".into(),
            changed_at: "2026-01-06T09:00:00Z".into(),
            unread: true,
        }
    );
    assert_eq!(inbox.rows[0].style, Style::Todo);
    assert_eq!(inbox.rows[0].status_name, "To Do");
}

#[tokio::test]
async fn notes_describe_what_changed() {
    let (_dir, db) = seeded().await;

    let (_, inbox) = open(&db).await;

    let notes: Vec<(&str, &str)> = inbox
        .rows
        .iter()
        .map(|row| (row.key.as_str(), row.note.as_str()))
        .collect();
    assert_eq!(
        notes,
        [
            ("NW-2", "1 new comment."),
            ("NW-3", "Marked unread."),
            ("NW-1", "Moved to Review."),
        ]
    );
}

#[tokio::test]
async fn a_row_serializes_with_camel_case_names() {
    let (_dir, db) = seeded().await;

    let (_, inbox) = open(&db).await;

    assert_eq!(
        serde_json::to_value(&inbox.rows[2]).unwrap(),
        serde_json::json!({
            "id": "20001",
            "key": "NW-1",
            "summary": "Mount the column",
            "projectId": "10001",
            "style": "review",
            "statusName": "Review",
            "note": "Moved to Review.",
            "changedAt": "2026-01-06T09:00:00Z",
            "unread": true,
        })
    );
}

#[tokio::test]
async fn an_issue_read_while_open_stays_listed_as_read() {
    let (_dir, db) = seeded().await;
    let (taken, _) = open(&db).await;

    mark_seen(&db, vec!["20002".into()], SEEN_LATER.into())
        .await
        .unwrap();
    let inbox = rows_of(&db, taken).await;

    assert_eq!(keys(&inbox), ["NW-2", "NW-3", "NW-1"]);
    assert_eq!(
        inbox.rows.iter().map(|row| row.unread).collect::<Vec<_>>(),
        [false, true, true]
    );
    assert_eq!(inbox.unread, 2);
}

#[tokio::test]
async fn a_new_snapshot_leaves_out_what_was_read() {
    let (_dir, db) = seeded().await;
    mark_seen(&db, vec!["20002".into()], SEEN_LATER.into())
        .await
        .unwrap();

    let (taken, inbox) = open(&db).await;

    assert_eq!(ids(&taken), ["20003", "20001"]);
    assert_eq!(keys(&inbox), ["NW-3", "NW-1"]);
}

#[tokio::test]
async fn an_issue_marked_unread_after_the_snapshot_joins_only_the_next_one() {
    let (_dir, db) = seeded().await;
    let (taken, _) = open(&db).await;

    mark_unread(&db, "20004".into(), None).await.unwrap();
    let old = rows_of(&db, taken).await;
    let (_, new) = open(&db).await;

    assert!(!keys(&old).contains(&"NW-4"));
    assert!(keys(&new).contains(&"NW-4"));
    let marked = new.rows.iter().find(|row| row.key == "NW-4").unwrap();
    assert_eq!(marked.note, "Marked unread.");
    assert!(marked.unread);
}

#[tokio::test]
async fn an_issue_marked_unread_after_its_changes_were_seen_is_ordered_by_its_updated_time() {
    let (_dir, db) = seeded().await;
    mark_unread(&db, "20004".into(), None).await.unwrap();

    let (taken, inbox) = open(&db).await;

    assert_eq!(ids(&taken), ["20002", "20004", "20003", "20001"]);
    let marked = inbox.rows.iter().find(|row| row.key == "NW-4").unwrap();
    assert_eq!(marked.changed_at, "2026-01-06T09:45:00Z");
}

#[tokio::test]
async fn an_issue_deleted_from_the_cache_after_the_snapshot_is_left_out() {
    let (_dir, db) = seeded().await;
    let (taken, _) = open(&db).await;

    db.write(|tx, _| Ok(tx.execute_batch("DELETE FROM cache.issue WHERE id = '20001';")?))
        .await
        .unwrap();
    let inbox = rows_of(&db, taken).await;

    assert_eq!(keys(&inbox), ["NW-2", "NW-3"]);
    assert_eq!(inbox.unread, 2);
}

#[tokio::test]
async fn an_issue_that_turned_gone_after_the_snapshot_is_left_out() {
    let (_dir, db) = seeded().await;
    let (taken, _) = open(&db).await;

    db.write(|tx, _| Ok(tx.execute_batch("UPDATE cache.issue SET gone = 1 WHERE id = '20003';")?))
        .await
        .unwrap();
    let inbox = rows_of(&db, taken).await;

    assert_eq!(keys(&inbox), ["NW-2", "NW-1"]);
}

#[tokio::test]
async fn count_is_the_number_of_unread_tracked_issues() {
    let (_dir, db) = seeded().await;

    assert_eq!(db.read(count).await.unwrap(), 3);

    mark_seen(&db, vec!["20002".into()], SEEN_LATER.into())
        .await
        .unwrap();
    mark_unread(&db, "20004".into(), None).await.unwrap();

    assert_eq!(db.read(count).await.unwrap(), 3);
}

#[tokio::test]
async fn count_is_zero_when_everything_is_read() {
    let (_dir, db) = seeded().await;

    mark_seen(
        &db,
        vec!["20001".into(), "20002".into(), "20003".into()],
        SEEN_LATER.into(),
    )
    .await
    .unwrap();

    assert_eq!(db.read(count).await.unwrap(), 0);
}
