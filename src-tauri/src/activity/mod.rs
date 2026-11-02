//! When the owner last viewed an issue and last did anything with it in the app (docs/data.md →
//! issue_activity): the Last view and Last interaction fields, and the Recently viewed page.

pub mod commands;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::db::{Changes, Db, LocalPart};
use crate::error::AppResult;
use crate::time::format_utc;

/// SQL for an issue's (alias `i`) last view and last interaction, as stored UTC text or null.
pub const LAST_VIEW: &str = "(SELECT a.viewed_at FROM issue_activity a WHERE a.issue_id = i.id)";
pub const LAST_INTERACTION: &str =
    "(SELECT a.interacted_at FROM issue_activity a WHERE a.issue_id = i.id)";

/// Opened in the Issue view: viewed, and so also used.
pub fn stamp_view(
    conn: &Connection,
    changes: &mut Changes,
    issue_id: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let now = format_utc(now);
    conn.execute(
        "INSERT INTO issue_activity (issue_id, viewed_at, interacted_at) VALUES (?1, ?2, ?2)
         ON CONFLICT(issue_id) DO UPDATE SET viewed_at = ?2, interacted_at = ?2",
        params![issue_id, now],
    )?;
    changes.local_changed(LocalPart::Activity);
    Ok(())
}

/// Previewed, bookmarked or changed through the app.
pub fn stamp_interaction(
    conn: &Connection,
    changes: &mut Changes,
    issue_id: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO issue_activity (issue_id, interacted_at) VALUES (?1, ?2)
         ON CONFLICT(issue_id) DO UPDATE SET interacted_at = ?2",
        params![issue_id, format_utc(now)],
    )?;
    changes.local_changed(LocalPart::Activity);
    Ok(())
}

pub async fn issue_viewed(db: &Db, issue_id: String, now: DateTime<Utc>) -> AppResult<()> {
    db.write(move |tx, changes| stamp_view(tx, changes, &issue_id, now))
        .await
}

pub async fn issue_previewed(db: &Db, issue_id: String, now: DateTime<Utc>) -> AppResult<()> {
    db.write(move |tx, changes| stamp_interaction(tx, changes, &issue_id, now))
        .await
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use chrono::TimeZone;
    use rusqlite::OptionalExtension;

    use super::*;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, hour, 30, 0).unwrap()
    }

    async fn lab() -> (tempfile::TempDir, Db) {
        let dir = crate::test_support::temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        (dir, db)
    }

    async fn stamps(db: &Db, issue_id: &str) -> Option<(Option<String>, Option<String>)> {
        let issue_id = issue_id.to_string();
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

    fn stamp(text: &str) -> Option<String> {
        Some(text.to_string())
    }

    #[tokio::test]
    async fn viewing_an_issue_stamps_both_the_view_and_the_interaction() {
        let (_dir, db) = lab().await;

        issue_viewed(&db, "10001".into(), at(9)).await.unwrap();

        assert_eq!(
            stamps(&db, "10001").await,
            Some((
                stamp("2026-10-28T09:30:00.000Z"),
                stamp("2026-10-28T09:30:00.000Z")
            ))
        );
    }

    #[tokio::test]
    async fn previewing_an_issue_stamps_only_the_interaction() {
        let (_dir, db) = lab().await;

        issue_previewed(&db, "10001".into(), at(9)).await.unwrap();

        assert_eq!(
            stamps(&db, "10001").await,
            Some((None, stamp("2026-10-28T09:30:00.000Z")))
        );
    }

    #[tokio::test]
    async fn previewing_a_viewed_issue_keeps_its_view_and_moves_the_interaction() {
        let (_dir, db) = lab().await;
        issue_viewed(&db, "10001".into(), at(9)).await.unwrap();

        issue_previewed(&db, "10001".into(), at(11)).await.unwrap();

        assert_eq!(
            stamps(&db, "10001").await,
            Some((
                stamp("2026-10-28T09:30:00.000Z"),
                stamp("2026-10-28T11:30:00.000Z")
            ))
        );
    }

    #[tokio::test]
    async fn a_later_view_overwrites_both_stamps() {
        let (_dir, db) = lab().await;
        issue_viewed(&db, "10001".into(), at(9)).await.unwrap();
        issue_previewed(&db, "10001".into(), at(10)).await.unwrap();

        issue_viewed(&db, "10001".into(), at(12)).await.unwrap();

        assert_eq!(
            stamps(&db, "10001").await,
            Some((
                stamp("2026-10-28T12:30:00.000Z"),
                stamp("2026-10-28T12:30:00.000Z")
            ))
        );
    }

    #[tokio::test]
    async fn issues_keep_their_own_stamps() {
        let (_dir, db) = lab().await;

        issue_viewed(&db, "10001".into(), at(9)).await.unwrap();
        issue_previewed(&db, "10002".into(), at(10)).await.unwrap();

        assert_eq!(
            stamps(&db, "10001").await.unwrap().0,
            stamp("2026-10-28T09:30:00.000Z")
        );
        assert_eq!(stamps(&db, "10002").await.unwrap().0, None);
        assert_eq!(stamps(&db, "10003").await, None);
    }

    #[tokio::test]
    async fn both_stamps_announce_the_activity_part() {
        let (_dir, db) = lab().await;

        let announced = db
            .write(|tx, _| {
                let mut view = Changes::default();
                stamp_view(tx, &mut view, "10001", at(9))?;
                let mut interaction = Changes::default();
                stamp_interaction(tx, &mut interaction, "10002", at(9))?;
                Ok((view, interaction))
            })
            .await
            .unwrap();

        for changes in [announced.0, announced.1] {
            assert_eq!(changes.local, BTreeSet::from([LocalPart::Activity]));
            assert!(changes.issues.is_empty());
        }
    }
}
