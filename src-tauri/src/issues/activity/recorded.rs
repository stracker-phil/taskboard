//! The field changes the sync recorded by comparing versions (`cache.issue_change`).

use rusqlite::{Connection, params};

use super::values::Values;
use super::{ActivityItem, ChangeValue};
use crate::error::AppResult;

/// `comment_added` and `mention` are left out: the comment itself is in the feed; so is `updated`, which says no more
/// than the history. Once the issue has history, changes at or before its newest entry are left out too (the
/// history has them, with authors), except `created`. A `people` change shows as its field, by name, set to the
/// account added.
pub fn items(
    conn: &Connection,
    id: &str,
    values: &Values,
    newest_history: Option<&str>,
) -> AppResult<Vec<ActivityItem>> {
    let mut query = conn.prepare(
        "SELECT c.field, c.old_value, c.new_value, c.changed_at, f.name
         FROM cache.issue_change c LEFT JOIN cache.field f ON c.field = 'people' AND f.id = c.old_value
         WHERE c.issue_id = ?1 AND c.field NOT IN ('comment_added', 'mention', 'updated')
           AND (c.field = 'created' OR ?2 IS NULL OR c.changed_at > ?2)",
    )?;
    let rows = query.query_map(params![id, newest_history], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
        ))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (field, old, new, at, people_field) = row?;
        let (field, from, to) = match field.as_str() {
            // The old value is the field's id, not a person.
            "people" => (
                people_field.or(old).unwrap_or(field),
                None,
                values.person_value(new.as_deref(), None),
            ),
            _ => {
                let (from, to) = (value(values, &field, old), value(values, &field, new));
                (field, from, to)
            }
        };
        items.push(ActivityItem::Change {
            field,
            from,
            to,
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
