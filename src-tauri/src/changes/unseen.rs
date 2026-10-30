//! Which recorded changes the owner hasn't seen: those after the issue's `issue_seen` time, or all of them
//! when it has none (the first sync marks every issue it finds as seen). An issue the owner marked unread
//! counts as unseen too, changes or not.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppResult;

/// An issue (alias `i`) with a change recorded after the owner last saw it, or marked unread.
pub const UNSEEN: &str = "(EXISTS (SELECT 1 FROM cache.issue_change c
    LEFT JOIN cache.issue_seen s ON s.issue_id = c.issue_id
    WHERE c.issue_id = i.id AND c.field != 'gone' AND (s.seen_at IS NULL OR c.changed_at > s.seen_at))
    OR EXISTS (SELECT 1 FROM cache.issue_seen u WHERE u.issue_id = i.id AND u.unread = 1))";

/// One row of `issue_change`.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    /// UTC, sortable as text.
    pub at: String,
}

fn change(r: &rusqlite::Row, from: usize) -> rusqlite::Result<Change> {
    Ok(Change {
        field: r.get(from)?,
        old_value: r.get(from + 1)?,
        new_value: r.get(from + 2)?,
        at: r.get(from + 3)?,
    })
}

/// The unseen changes of the issues `ids`, oldest first; an issue marked unread without any has an empty list.
pub fn unseen_changes(conn: &Connection, ids: &[&str]) -> AppResult<HashMap<String, Vec<Change>>> {
    let ids = serde_json::to_string(ids)?;
    let mut query = conn.prepare(
        "SELECT c.issue_id, c.field, c.old_value, c.new_value, c.changed_at
         FROM json_each(?1) j JOIN cache.issue_change c ON c.issue_id = j.value
         LEFT JOIN cache.issue_seen s ON s.issue_id = c.issue_id
         WHERE c.field != 'gone' AND (s.seen_at IS NULL OR c.changed_at > s.seen_at)
         ORDER BY c.changed_at, c.id",
    )?;
    let rows = query.query_map([&ids], |r| Ok((r.get::<_, String>(0)?, change(r, 1)?)))?;
    let mut by_issue: HashMap<String, Vec<Change>> = HashMap::new();
    for row in rows {
        let (id, change) = row?;
        by_issue.entry(id).or_default().push(change);
    }
    let mut marked = conn.prepare(
        "SELECT s.issue_id FROM json_each(?1) j JOIN cache.issue_seen s ON s.issue_id = j.value
         WHERE s.unread = 1",
    )?;
    for id in marked.query_map([&ids], |r| r.get::<_, String>(0))? {
        by_issue.entry(id?).or_default();
    }
    Ok(by_issue)
}

/// One issue's changes after `after`, oldest first; "" returns them all.
pub fn changes_after(conn: &Connection, id: &str, after: &str) -> AppResult<Vec<Change>> {
    let mut query = conn.prepare(
        "SELECT field, old_value, new_value, changed_at FROM cache.issue_change
         WHERE issue_id = ?1 AND field != 'gone' AND changed_at > ?2
         ORDER BY changed_at, id",
    )?;
    let rows = query.query_map(params![id, after], |r| change(r, 0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Where new activity starts while the issue has unseen changes: the last look, or "" when never seen.
pub fn unseen_since(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    let unseen: bool = conn.query_row(
        &format!("SELECT EXISTS (SELECT 1 FROM cache.issue i WHERE i.id = ?1 AND {UNSEEN})"),
        [id],
        |r| r.get(0),
    )?;
    Ok(if unseen {
        Some(seen_at(conn, id)?.unwrap_or_default())
    } else {
        None
    })
}

/// When the owner last saw the issue; none when never.
pub fn seen_at(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT seen_at FROM cache.issue_seen WHERE issue_id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::test_support::temp_config_dir;

    const SETUP: &str = "
        INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}');
        INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id, status_category,
          created_at, updated_at, raw, synced_at)
        VALUES ('20001', 'NW-1', '10001', 'Mount the column', 'Task', '1', 'new',
          '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '{}', '2026-01-05T09:00:00Z'),
         ('20002', 'NW-2', '10001', 'Paint the wall', 'Task', '1', 'new',
          '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '{}', '2026-01-05T09:00:00Z'),
         ('20003', 'NW-3', '10001', 'Hang the door', 'Task', '1', 'new',
          '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '{}', '2026-01-05T09:00:00Z');
        INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at) VALUES
          ('20001', 'status', '1', '2', '2026-01-06T09:00:00Z'),
          ('20002', 'status', '1', '3', '2026-01-06T10:00:00Z');
        INSERT INTO cache.issue_seen (issue_id, seen_at, unread) VALUES
          ('20003', '2026-01-05T09:00:00Z', 1);";

    fn change(field: &str, old: &str, new: &str, at: &str) -> Change {
        Change {
            field: field.into(),
            old_value: Some(old.into()),
            new_value: Some(new.into()),
            at: at.into(),
        }
    }

    async fn unseen_of(ids: &'static [&'static str]) -> HashMap<String, Vec<Change>> {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| Ok(tx.execute_batch(SETUP)?))
            .await
            .unwrap();
        db.read(move |conn| unseen_changes(conn, ids))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn only_the_changes_of_the_asked_issues_are_returned() {
        let unseen = unseen_of(&["20001"]).await;

        assert_eq!(
            unseen,
            HashMap::from([(
                "20001".to_string(),
                vec![change("status", "1", "2", "2026-01-06T09:00:00Z")]
            )])
        );
    }

    #[tokio::test]
    async fn an_issue_marked_unread_is_listed_without_changes_when_asked_for() {
        let unseen = unseen_of(&["20002", "20003"]).await;

        assert_eq!(unseen.len(), 2);
        assert_eq!(
            unseen["20002"],
            vec![change("status", "1", "3", "2026-01-06T10:00:00Z")]
        );
        assert_eq!(unseen["20003"], vec![]);
    }

    #[tokio::test]
    async fn no_ids_return_nothing() {
        assert!(unseen_of(&[]).await.is_empty());
    }
}
