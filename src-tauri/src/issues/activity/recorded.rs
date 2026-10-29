//! The field changes the sync recorded by comparing versions (`cache.issue_change`).

use rusqlite::{Connection, params};

use super::values::Values;
use super::{ActivityItem, ChangeValue};
use crate::error::AppResult;

/// `comment_added` is left out: the comment itself is in the feed. Once the issue has history, changes at or
/// before its newest entry are left out too (the history has them, with authors), except `created`.
pub fn items(
    conn: &Connection,
    id: &str,
    values: &Values,
    newest_history: Option<&str>,
) -> AppResult<Vec<ActivityItem>> {
    let mut query = conn.prepare(
        "SELECT field, old_value, new_value, changed_at FROM cache.issue_change
         WHERE issue_id = ?1 AND field != 'comment_added'
           AND (field = 'created' OR ?2 IS NULL OR changed_at > ?2)",
    )?;
    let rows = query.query_map(params![id, newest_history], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (field, old, new, at) = row?;
        items.push(ActivityItem::Change {
            from: value(values, &field, old),
            to: value(values, &field, new),
            field,
            author: None,
            at,
            fresh: false,
        });
    }
    Ok(items)
}

fn value(values: &Values, field: &str, raw: Option<String>) -> Option<ChangeValue> {
    let raw = raw?;
    match field {
        "status" => values.status(Some(&raw), None),
        "assignee" => values.person_value(Some(&raw), None),
        _ => Some(ChangeValue::Text(raw)),
    }
}
