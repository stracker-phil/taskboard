//! `cache.user`: every writer stores people through here, so a known account type is never lost to an answer
//! that leaves it out.

use rusqlite::{Connection, params};

use super::map::UserRow;
use crate::error::AppResult;

/// Inserts or updates the user. Returns whether the row changed.
pub fn upsert_user(conn: &Connection, user: &UserRow) -> AppResult<bool> {
    let changed = conn.execute(
        "INSERT INTO cache.user (account_id, name, avatar_url, account_type) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (account_id) DO UPDATE SET name = excluded.name, avatar_url = excluded.avatar_url,
           account_type = COALESCE(excluded.account_type, account_type)
         WHERE name IS NOT excluded.name OR avatar_url IS NOT excluded.avatar_url
           OR account_type IS NOT COALESCE(excluded.account_type, account_type)",
        params![
            user.account_id,
            user.name,
            user.avatar_url,
            user.account_type
        ],
    )?;
    Ok(changed > 0)
}

/// Adds the account by name unless the cache knows it already.
pub fn remember_user(conn: &Connection, account_id: &str, name: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.user (account_id, name) VALUES (?1, ?2) ON CONFLICT (account_id) DO NOTHING",
        params![account_id, name],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::test_support::temp_config_dir;
    use tempfile::TempDir;

    const RUNNER: &str = "account-test-runner";

    fn user(account_type: Option<&str>) -> UserRow {
        UserRow {
            account_id: RUNNER.into(),
            name: "Test Runner".into(),
            avatar_url: None,
            account_type: account_type.map(String::from),
        }
    }

    fn open() -> (TempDir, Db) {
        let dir = temp_config_dir();
        let db = Db::open(dir.path()).unwrap();
        (dir, db)
    }

    async fn upsert(db: &Db, user: UserRow) -> bool {
        db.write(move |tx, _| upsert_user(tx, &user)).await.unwrap()
    }

    async fn stored(db: &Db) -> (String, Option<String>) {
        db.read(|conn| {
            Ok(conn.query_row(
                "SELECT name, account_type FROM cache.user WHERE account_id = ?1",
                [RUNNER],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_new_user_is_stored_with_its_account_type() {
        let (_dir, db) = open();

        assert!(upsert(&db, user(Some("app"))).await);

        assert_eq!(
            stored(&db).await,
            ("Test Runner".into(), Some("app".into()))
        );
    }

    #[tokio::test]
    async fn a_later_row_without_an_account_type_keeps_the_known_one() {
        let (_dir, db) = open();
        upsert(&db, user(Some("app"))).await;

        let renamed = UserRow {
            name: "Test Runner 2".into(),
            ..user(None)
        };

        assert!(upsert(&db, renamed).await);
        assert_eq!(
            stored(&db).await,
            ("Test Runner 2".into(), Some("app".into()))
        );
    }

    #[tokio::test]
    async fn a_new_account_type_overwrites_the_known_one() {
        let (_dir, db) = open();
        upsert(&db, user(Some("atlassian"))).await;

        assert!(upsert(&db, user(Some("app"))).await);

        assert_eq!(stored(&db).await.1, Some("app".into()));
    }

    #[tokio::test]
    async fn a_known_account_type_is_filled_in_for_a_user_stored_without_one() {
        let (_dir, db) = open();
        upsert(&db, user(None)).await;

        assert!(upsert(&db, user(Some("app"))).await);

        assert_eq!(stored(&db).await.1, Some("app".into()));
    }

    #[tokio::test]
    async fn an_identical_row_does_not_change_the_user() {
        let (_dir, db) = open();
        upsert(&db, user(Some("app"))).await;

        assert!(!upsert(&db, user(Some("app"))).await);
        assert!(!upsert(&db, user(None)).await);
        assert_eq!(stored(&db).await.1, Some("app".into()));
    }
}
