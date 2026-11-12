//! The record of every change sent to Jira and Jira's answer (docs/writes.md).

use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, params};

use crate::activity;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::time::format_utc;

/// How long a record is kept.
pub const KEEP_DAYS: i64 = 180;

#[derive(Clone, Debug, PartialEq)]
pub struct SentChange {
    /// When it was sent (UTC).
    pub at: String,
    /// `None` when the issue had left the cache.
    pub issue_key: Option<String>,
    /// "Status: Done", `Comment: "Looks good…" (42 characters)`.
    pub change: String,
    /// Jira's refusal; `None` when Jira took it.
    pub error: Option<String>,
}

/// Typed text as the record keeps it: its first 10 characters and its length, never all of it
/// (`.claude/rules/no-issue-data-on-disk.md`).
pub fn typed(kind: &str, text: &str) -> String {
    const SHOWN: usize = 10;
    let count = text.chars().count();
    let start: String = text.chars().take(SHOWN).collect();
    let more = if count > SHOWN { "…" } else { "" };
    format!("{kind}: \"{start}{more}\" ({count} characters)")
}

/// Records a change Jira was asked for, with its refusal if any; a change Jira took counts as using the issue. A
/// record that can't be stored doesn't fail the change.
pub async fn record(db: &Db, issue_id: &str, change: String, refusal: Option<&AppError>) {
    let issue_id = issue_id.to_string();
    let error = refusal.map(AppError::to_string);
    let now = Utc::now();
    let stored = db
        .write(move |tx, changes| {
            insert(tx, now, &issue_id, &change, error.as_deref())?;
            if error.is_none() {
                activity::stamp_interaction(tx, changes, &issue_id, now)?;
            }
            Ok(())
        })
        .await;
    if let Err(e) = stored {
        log::warn!("A change sent to Jira wasn't recorded: {e}");
    }
}

/// Adds one record and drops the ones past their time.
pub fn insert(
    conn: &Connection,
    now: DateTime<Utc>,
    issue_id: &str,
    change: &str,
    error: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO write_log (at, issue_id, issue_key, change, error)
         VALUES (?1, ?2, (SELECT key FROM cache.issue WHERE id = ?2), ?3, ?4)",
        params![format_utc(now), issue_id, change, error],
    )?;
    prune(conn, now)
}

fn prune(conn: &Connection, now: DateTime<Utc>) -> AppResult<()> {
    let oldest = now - Duration::days(KEEP_DAYS);
    conn.execute("DELETE FROM write_log WHERE at < ?1", [format_utc(oldest)])?;
    Ok(())
}

/// The newest records first; `limit` of them, or all.
pub fn sent_changes(conn: &Connection, limit: Option<u32>) -> AppResult<Vec<SentChange>> {
    let mut query = conn.prepare(
        "SELECT at, issue_key, change, error FROM write_log ORDER BY at DESC, id DESC LIMIT ?1",
    )?;
    let rows = query.query_map([limit.map_or(-1, i64::from)], |row| {
        Ok(SentChange {
            at: row.get(0)?,
            issue_key: row.get(1)?,
            change: row.get(2)?,
            error: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use chrono::TimeZone;

    fn noon() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap()
    }

    fn change(at: &str, key: Option<&str>, text: &str, error: Option<&str>) -> SentChange {
        SentChange {
            at: at.into(),
            issue_key: key.map(Into::into),
            change: text.into(),
            error: error.map(Into::into),
        }
    }

    async fn db_with_cached_issue() -> (tempfile::TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        db.write(|tx, _| {
            tx.execute_batch(
                "INSERT INTO cache.space (id, key, name, raw) VALUES ('10001', 'NW', 'Northwind', '{}');
                 INSERT INTO cache.issue (id, key, project_id, summary, issue_type, status_id,
                   status_category, created_at, updated_at, synced_at)
                 VALUES ('20001', 'NW-1', '10001', 'Mount the column', 'Task', '1', 'new',
                   '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z', '2026-01-05T09:00:00Z');",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        (dir, db)
    }

    async fn insert_at(db: &Db, at: DateTime<Utc>, issue_id: &'static str, text: &'static str) {
        db.write(move |tx, _| insert(tx, at, issue_id, text, None))
            .await
            .unwrap();
    }

    async fn all(db: &Db) -> Vec<SentChange> {
        db.read(|conn| sent_changes(conn, None)).await.unwrap()
    }

    #[tokio::test]
    async fn a_record_takes_the_issue_key_from_the_cache() {
        let (_dir, db) = db_with_cached_issue().await;

        db.write(|tx, _| insert(tx, noon(), "20001", "Status: Done", None))
            .await
            .unwrap();

        assert_eq!(
            all(&db).await,
            [change(
                "2026-10-28T12:00:00.000Z",
                Some("NW-1"),
                "Status: Done",
                None
            )]
        );
    }

    #[tokio::test]
    async fn a_record_of_an_issue_that_is_not_cached_has_no_key_and_keeps_jiras_refusal() {
        let (_dir, db) = db_with_cached_issue().await;

        db.write(|tx, _| insert(tx, noon(), "99999", "Priority: Low", Some("Not allowed")))
            .await
            .unwrap();

        assert_eq!(
            all(&db).await,
            [change(
                "2026-10-28T12:00:00.000Z",
                None,
                "Priority: Low",
                Some("Not allowed")
            )]
        );
    }

    #[tokio::test]
    async fn records_older_than_180_days_are_dropped_when_a_new_one_is_added() {
        let (_dir, db) = db_with_cached_issue().await;
        insert_at(&db, noon() - Duration::days(181), "20001", "too old").await;
        insert_at(&db, noon() - Duration::days(180), "20001", "on the edge").await;
        insert_at(&db, noon() - Duration::days(179), "20001", "recent").await;

        insert_at(&db, noon(), "20001", "new").await;

        let kept: Vec<String> = all(&db).await.into_iter().map(|c| c.change).collect();
        assert_eq!(kept, ["new", "recent", "on the edge"]);
    }

    #[tokio::test]
    async fn the_newest_come_first_and_the_same_moment_goes_by_the_order_added() {
        let (_dir, db) = db_with_cached_issue().await;
        insert_at(&db, noon(), "20001", "second of the moment").await;
        insert_at(&db, noon() - Duration::hours(1), "20001", "oldest").await;
        insert_at(&db, noon(), "20001", "third of the moment").await;

        let order: Vec<String> = all(&db).await.into_iter().map(|c| c.change).collect();

        assert_eq!(
            order,
            ["third of the moment", "second of the moment", "oldest"]
        );
    }

    #[tokio::test]
    async fn a_limit_keeps_only_the_newest_and_none_returns_all() {
        let (_dir, db) = db_with_cached_issue().await;
        for (hours, text) in [(3, "a"), (2, "b"), (1, "c")] {
            insert_at(&db, noon() - Duration::hours(hours), "20001", text).await;
        }

        let newest_two: Vec<String> = db
            .read(|conn| sent_changes(conn, Some(2)))
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.change)
            .collect();

        assert_eq!(newest_two, ["c", "b"]);
        assert_eq!(all(&db).await.len(), 3);
    }

    async fn activity_of(
        db: &Db,
        issue_id: &'static str,
    ) -> Option<(Option<String>, Option<String>)> {
        use rusqlite::OptionalExtension;
        db.read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT viewed_at, interacted_at FROM issue_activity WHERE issue_id = ?1",
                    [issue_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?)
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_change_jira_took_counts_as_an_interaction_but_not_a_view() {
        let (_dir, db) = db_with_cached_issue().await;

        record(&db, "20001", "Status: Done".into(), None).await;

        let (viewed, interacted) = activity_of(&db, "20001").await.unwrap();
        assert_eq!(viewed, None);
        assert!(interacted.is_some());
    }

    #[tokio::test]
    async fn a_change_jira_refused_is_not_an_interaction() {
        let (_dir, db) = db_with_cached_issue().await;
        let refusal = AppError::InvalidInput("Not allowed".into());

        record(&db, "20001", "Status: Done".into(), Some(&refusal)).await;

        assert_eq!(all(&db).await.len(), 1);
        assert_eq!(activity_of(&db, "20001").await, None);
    }

    #[test]
    fn typed_text_keeps_its_first_ten_characters_and_its_length() {
        for (kind, text, expected) in [
            ("Comment", "", "Comment: \"\" (0 characters)"),
            (
                "Comment",
                "Looks good",
                "Comment: \"Looks good\" (10 characters)",
            ),
            (
                "Comment",
                "Looks good to me",
                "Comment: \"Looks good…\" (16 characters)",
            ),
            (
                "Title",
                "Säule montieren bitte",
                "Title: \"Säule mont…\" (21 characters)",
            ),
            (
                "Web link",
                "https://example.com/spec",
                "Web link: \"https://ex…\" (24 characters)",
            ),
        ] {
            assert_eq!(typed(kind, text), expected, "{text}");
        }
    }

    #[tokio::test]
    async fn recording_stores_the_change_and_a_refusals_message_and_never_fails() {
        let (_dir, db) = db_with_cached_issue().await;

        record(&db, "20001", typed("Comment", "Looks good"), None).await;
        let refusal = AppError::InvalidInput("Not allowed".into());
        record(&db, "20001", "Priority: Low".into(), Some(&refusal)).await;

        let stored = all(&db).await;
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[1].change, "Comment: \"Looks good\" (10 characters)");
        assert_eq!(stored[1].issue_key.as_deref(), Some("NW-1"));
        assert_eq!(stored[1].error, None);
        assert_eq!(stored[0].change, "Priority: Low");
        assert_eq!(stored[0].error, Some(refusal.to_string()));
    }
}
