//! The issue's history (the tracker's changelog) as activity: one change per changed field, with its author.

use rusqlite::Connection;

use super::values::Values;
use super::{ActivityItem, text};
use crate::error::AppResult;
use crate::issues::history::{HistoryRow, rows};

/// Longer values aren't shown; the change just names the field.
const LONGEST_VALUE: usize = 120;
/// Fields whose values are long text, never shown.
const LONG_TEXT_FIELDS: &[&str] = &["description", "environment"];

pub fn items(conn: &Connection, id: &str, values: &Values) -> AppResult<Vec<ActivityItem>> {
    Ok(rows(conn, id)?
        .into_iter()
        .map(|row| item(row, values))
        .collect())
}

fn item(row: HistoryRow, values: &Values) -> ActivityItem {
    let key = row
        .field_id
        .clone()
        .unwrap_or_else(|| row.field.clone())
        .to_lowercase();
    let author = row
        .author_id
        .as_deref()
        .map(|author| values.person(author, None));
    let HistoryRow {
        field,
        from_value,
        from_text,
        to_value,
        to_text,
        changed_at,
        ..
    } = row;
    let (field, from, to) = match key.as_str() {
        "status" => (
            key,
            values.status(from_value.as_deref(), from_text.as_deref()),
            values.status(to_value.as_deref(), to_text.as_deref()),
        ),
        "assignee" | "reporter" => (
            key,
            values.person_value(from_value.as_deref(), from_text.as_deref()),
            values.person_value(to_value.as_deref(), to_text.as_deref()),
        ),
        "priority" => (
            key,
            text(from_text.or(from_value)),
            text(to_text.or(to_value)),
        ),
        "duedate" => (
            "due_date".to_owned(),
            text(from_value.or(from_text)),
            text(to_value.or(to_text)),
        ),
        _ => {
            let from = from_text.or(from_value);
            let to = to_text.or(to_value);
            if is_long(&key, from.as_deref(), to.as_deref()) {
                (field, None, None)
            } else {
                (field, text(from), text(to))
            }
        }
    };
    ActivityItem::Change {
        field,
        from,
        to,
        author,
        at: changed_at,
        fresh: false,
    }
}

fn is_long(key: &str, from: Option<&str>, to: Option<&str>) -> bool {
    LONG_TEXT_FIELDS.contains(&key)
        || [from, to]
            .into_iter()
            .flatten()
            .any(|value| value.chars().count() > LONGEST_VALUE)
}
