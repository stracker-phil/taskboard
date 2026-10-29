//! The avatar rows in the cache: whose address the sync saw, and the pictures that arrived.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use super::freshness::{Cached, Picture};
use crate::error::AppResult;

/// The address the sync stored for the person; `None` for an unknown person or one without a picture.
pub fn wanted_url(conn: &Connection, account_id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT avatar_url FROM cache.user WHERE account_id = ?1",
            [account_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
}

pub fn cached(conn: &Connection, account_id: &str) -> AppResult<Option<Cached>> {
    let row = conn
        .query_row(
            "SELECT url, mime, bytes, fetched_at, failed_at FROM cache.avatar WHERE account_id = ?1",
            [account_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<Vec<u8>>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()?;
    Ok(row.map(|(url, mime, bytes, fetched_at, failed_at)| Cached {
        url,
        picture: mime.zip(bytes).map(|(mime, bytes)| Picture { mime, bytes }),
        fetched_at: fetched_at.as_deref().and_then(instant),
        failed_at: failed_at.as_deref().and_then(instant),
    }))
}

pub fn save_picture(
    conn: &Connection,
    account_id: &str,
    url: &str,
    picture: &Picture,
    now: DateTime<Utc>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.avatar (account_id, url, mime, bytes, fetched_at, failed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL)
         ON CONFLICT (account_id) DO UPDATE SET url = excluded.url, mime = excluded.mime,
           bytes = excluded.bytes, fetched_at = excluded.fetched_at, failed_at = NULL",
        params![
            account_id,
            url,
            picture.mime,
            picture.bytes,
            now.to_rfc3339()
        ],
    )?;
    Ok(())
}

/// Remembers the failure for `url`; the last picture that arrived stays.
pub fn save_failure(
    conn: &Connection,
    account_id: &str,
    url: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO cache.avatar (account_id, url, failed_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (account_id) DO UPDATE SET url = excluded.url, failed_at = excluded.failed_at",
        params![account_id, url, now.to_rfc3339()],
    )?;
    Ok(())
}

fn instant(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|at| at.with_timezone(&Utc))
}
