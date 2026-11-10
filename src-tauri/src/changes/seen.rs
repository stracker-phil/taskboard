//! Marking issues read (seen) by the owner or after the shown issue's delay, and unread again.

use chrono::{DateTime, Duration};
use rusqlite::{Connection, params};

use super::unseen::is_unseen;
use crate::db::{Changes, Db};
use crate::error::{AppError, AppResult};
use crate::time::format_utc;

/// Seen at `now`, or at the issue's newest change or comment when Jira's clock runs ahead of this Mac's, so
/// nothing already shown stays unseen. Only an issue that was unseen counts as changed: reading a read issue
/// again shows nothing new.
fn mark(conn: &Connection, changes: &mut Changes, ids: &[String], now: &str) -> AppResult<()> {
    let mut insert = conn.prepare(
        "INSERT INTO issue_seen (issue_id, seen_at)
         SELECT i.id, MAX(?2,
           COALESCE((SELECT MAX(changed_at) FROM cache.issue_change WHERE issue_id = i.id), ''),
           COALESCE((SELECT MAX(created_at) FROM cache.comment WHERE issue_id = i.id), ''))
         FROM cache.issue i WHERE i.id = ?1
         ON CONFLICT (issue_id) DO UPDATE SET seen_at = MAX(seen_at, excluded.seen_at), unread = 0",
    )?;
    for id in ids {
        let was_unseen = is_unseen(conn, id)?;
        insert.execute(params![id, now])?;
        if was_unseen {
            changes.issue_changed(id);
        }
    }
    Ok(())
}

/// Seen up to the issue's `updated`, only forward; an issue marked unread by hand keeps its mark and time.
pub fn seen_until_updated(conn: &Connection, changes: &mut Changes, id: &str) -> AppResult<()> {
    let moved = conn.execute(
        "INSERT INTO issue_seen (issue_id, seen_at)
         SELECT id, updated_at FROM cache.issue WHERE id = ?1
         ON CONFLICT (issue_id) DO UPDATE SET seen_at = excluded.seen_at
         WHERE unread = 0 AND seen_at < excluded.seen_at",
        [id],
    )?;
    if moved > 0 {
        changes.issue_changed(id);
    }
    Ok(())
}

/// `seen_until_updated` for an issue without unseen changes, so a later look at `updated` alone finds it read.
pub fn catch_up(conn: &Connection, changes: &mut Changes, id: &str) -> AppResult<()> {
    if is_unseen(conn, id)? {
        return Ok(());
    }
    seen_until_updated(conn, changes, id)
}

pub async fn mark_seen(db: &Db, ids: Vec<String>, now: String) -> AppResult<()> {
    db.write(move |tx, changes| mark(tx, changes, &ids, &now))
        .await
}

/// Unread again. With `from` (an activity item's time, RFC 3339) that item and everything newer count as new;
/// without, the issue is unread and its activity keeps its state.
pub async fn mark_unread(db: &Db, id: String, from: Option<String>) -> AppResult<()> {
    let before = match from {
        Some(at) => Some(just_before(&at)?),
        None => None,
    };
    db.write(move |tx, changes| unread(tx, changes, &id, before.as_deref()))
        .await
}

/// Read when it is unread, else unread again as a whole.
pub async fn toggle_seen(db: &Db, id: String, now: String) -> AppResult<()> {
    db.write(move |tx, changes| match is_unseen(tx, &id)? {
        true => mark(tx, changes, std::slice::from_ref(&id), &now),
        false => unread(tx, changes, &id, None),
    })
    .await
}

fn unread(
    conn: &Connection,
    changes: &mut Changes,
    id: &str,
    before: Option<&str>,
) -> AppResult<()> {
    let marked = conn.execute(
        "INSERT INTO issue_seen (issue_id, seen_at, unread)
         SELECT id, COALESCE(?2, ''), 1 FROM cache.issue WHERE id = ?1
         ON CONFLICT (issue_id) DO UPDATE SET unread = 1,
           seen_at = CASE WHEN ?2 IS NULL THEN seen_at ELSE MIN(seen_at, ?2) END",
        params![id, before],
    )?;
    if marked > 0 {
        changes.issue_changed(id);
    }
    Ok(())
}

fn just_before(at: &str) -> AppResult<String> {
    let at = DateTime::parse_from_rfc3339(at)
        .map_err(|e| AppError::InvalidInput(format!("“{at}” isn't a time: {e}")))?;
    Ok(format_utc(at.to_utc() - Duration::milliseconds(1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;

    const NOW: &str = "2026-01-07T00:00:00.000Z";

    const SETUP: &str = "
        INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}');
        INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
          created_at, updated_at, synced_at)
        VALUES ('20001', 'NW-1', '10001', 'Mount the column', 'Task', '1', 'new',
          '2026-01-05T09:00:00Z', '2026-01-06T09:00:00Z', '2026-01-06T09:00:00Z'),
         ('20002', 'NW-2', '10001', 'Paint the wall', 'Task', '1', 'new',
          '2026-01-05T09:00:00Z', '2026-01-06T10:00:00Z', '2026-01-06T10:00:00Z');
        INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at) VALUES
          ('20001', 'status', '1', '2', '2026-01-06T09:00:00.000Z'),
          ('20002', 'status', '1', '2', '2026-01-06T10:00:00.000Z');
        INSERT INTO issue_seen (issue_id, seen_at, unread) VALUES
          ('20001', '2026-01-05T09:00:00.000Z', 0),
          ('20002', '2026-01-06T11:00:00.000Z', 0);";

    const UNREAD: &str = "20001";
    const READ: &str = "20002";

    async fn seeded() -> (tempfile::TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| Ok(tx.execute_batch(SETUP)?))
            .await
            .unwrap();
        (dir, db)
    }

    async fn state(db: &Db, id: &str) -> (bool, i64) {
        let id = id.to_string();
        db.read(move |conn| {
            let flag = conn.query_row(
                "SELECT unread FROM issue_seen WHERE issue_id = ?1",
                [&id],
                |r| r.get(0),
            )?;
            Ok((is_unseen(conn, &id)?, flag))
        })
        .await
        .unwrap()
    }

    async fn toggle(db: &Db, id: &str) {
        toggle_seen(db, id.into(), NOW.into()).await.unwrap();
    }

    #[tokio::test]
    async fn toggling_an_unread_issue_reads_it() {
        let (_dir, db) = seeded().await;
        assert_eq!(state(&db, UNREAD).await, (true, 0));

        toggle(&db, UNREAD).await;

        assert_eq!(state(&db, UNREAD).await, (false, 0));
    }

    #[tokio::test]
    async fn toggling_a_read_issue_marks_it_unread() {
        let (_dir, db) = seeded().await;
        assert_eq!(state(&db, READ).await, (false, 0));

        toggle(&db, READ).await;

        assert_eq!(state(&db, READ).await, (true, 1));
    }

    #[tokio::test]
    async fn toggling_twice_returns_the_issue_to_where_it_started() {
        for (id, started_unread) in [(UNREAD, true), (READ, false)] {
            let (_dir, db) = seeded().await;

            toggle(&db, id).await;
            toggle(&db, id).await;

            let (unseen, _) = state(&db, id).await;
            assert_eq!(unseen, started_unread, "{id}");
        }
    }
}
