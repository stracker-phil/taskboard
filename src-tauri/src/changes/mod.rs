//! What changed since the owner last looked: unseen changes, the words for them, and marking issues seen.

pub mod commands;
mod facts;
mod seen;
mod unseen;

use chrono::Duration;
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

pub use facts::{Names, chip, facts};
pub use seen::{mark_all_seen, mark_seen, mark_unread};
pub use unseen::{UNSEEN, seen_at, unseen_changes, unseen_since};

use facts::sentence;
use unseen::{Change, changes_after};

use crate::error::AppResult;
use crate::lists::Viewer;
use crate::sync::map::format_utc;

/// The banner over an issue with unseen changes.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Since {
    /// "Since yesterday", or "Since you last looked" when that was longer ago.
    pub label: String,
    /// "2 new comments, moved to Review."
    pub text: String,
}

/// Changes over the last day or since the owner last saw the issue, whichever is longer; none without unseen
/// changes.
pub fn since(
    conn: &Connection,
    viewer: &Viewer,
    id: &str,
    names: &Names,
) -> AppResult<Option<Since>> {
    let seen = seen_at(conn, id)?;
    let day_ago = format_utc(viewer.now - Duration::days(1));
    // Never seen: every recorded change counts.
    let start = seen.as_deref().map_or("", |s| s.min(day_ago.as_str()));
    let changes = changes_after(conn, id, start)?;
    let unseen = |c: &Change| seen.as_deref().is_none_or(|s| c.at.as_str() > s);
    if !changes.iter().any(unseen) {
        return Ok(None);
    }
    let older_than_a_day = changes.iter().any(|c| c.at < day_ago);
    Ok(sentence(&facts(&changes), names).map(|text| Since {
        label: if older_than_a_day {
            "Since you last looked"
        } else {
            "Since yesterday"
        }
        .into(),
        text,
    }))
}
