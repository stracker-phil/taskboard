//! Mentions in comments the cache hasn't seen yet, recorded as `mention` changes: one per mentioned account, so
//! Unread can say "Mentioned you" without the sync knowing who "you" is.

use std::collections::HashSet;

use rusqlite::{Connection, params};
use serde_json::Value;

use super::map::Mapped;
use crate::adf;
use crate::error::AppResult;

pub fn record_mentions(conn: &Connection, mapped: &Mapped) -> AppResult<()> {
    let issue_id = &mapped.issue.id;
    let mut query = conn.prepare_cached("SELECT id FROM cache.comment WHERE issue_id = ?1")?;
    let known: HashSet<String> = query
        .query_map([issue_id], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for comment in mapped.comments.iter().filter(|c| !known.contains(&c.id)) {
        let Some(doc) = comment
            .body_adf
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok())
        else {
            continue;
        };
        for account in adf::mentions(&doc) {
            conn.execute(
                "INSERT INTO cache.issue_change (issue_id, field, old_value, new_value, changed_at)
                 VALUES (?1, 'mention', ?2, ?3, ?4)",
                params![issue_id, comment.id, account, comment.created_at],
            )?;
        }
    }
    Ok(())
}
