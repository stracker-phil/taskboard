//! When an opened issue's on-demand details (web links, history) were last fetched: moving through issues
//! doesn't ask the tracker for each again within `FRESH_FOR`.

use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::db::Db;
use crate::error::AppResult;

pub const FRESH_FOR: Duration = Duration::minutes(5);

#[derive(Clone, Copy, Debug)]
pub enum Detail {
    WebLinks,
    History,
}

impl Detail {
    fn column(self) -> &'static str {
        match self {
            Self::WebLinks => "links_synced_at",
            Self::History => "history_synced_at",
        }
    }
}

/// Whether to fetch the detail at `now`: the issue is cached, not gone, and the detail is older than
/// `FRESH_FOR` or was never fetched.
pub async fn due(db: &Db, id: &str, detail: Detail, now: DateTime<Utc>) -> AppResult<bool> {
    let id = id.to_owned();
    let Some(fetched_at) = db.read(move |conn| last_fetched(conn, &id, detail)).await? else {
        return Ok(false);
    };
    Ok(fetched_at.is_none_or(|at| now - at >= FRESH_FOR))
}

pub fn mark_fetched(
    conn: &Connection,
    id: &str,
    detail: Detail,
    now: DateTime<Utc>,
) -> AppResult<()> {
    conn.execute(
        &format!(
            "UPDATE cache.issue SET {} = ?2 WHERE id = ?1",
            detail.column()
        ),
        params![id, now.to_rfc3339()],
    )?;
    Ok(())
}

/// None when the issue isn't cached (or is gone); else when the detail was last fetched, if ever.
fn last_fetched(
    conn: &Connection,
    id: &str,
    detail: Detail,
) -> AppResult<Option<Option<DateTime<Utc>>>> {
    let at: Option<Option<String>> = conn
        .query_row(
            &format!(
                "SELECT {} FROM cache.issue WHERE id = ?1 AND gone = 0",
                detail.column()
            ),
            [id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(at.map(|at| {
        at.and_then(|at| DateTime::parse_from_rfc3339(&at).ok())
            .map(|at| at.with_timezone(&Utc))
    }))
}
