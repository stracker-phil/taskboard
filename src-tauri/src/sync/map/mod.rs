//! Jira's issue JSON → the cache's rows. Pure: no database, no clock.

mod comments;
mod issue;
#[cfg(test)]
mod tests;
mod users;

use serde_json::Value;

pub use comments::CommentRow;
pub use issue::{IssueRow, map_issue};
pub use users::UserRow;

/// The custom fields whose ids differ per site (found in `/field` by their type).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldIds {
    pub sprint: Option<String>,
    pub epic_color: Option<String>,
}

/// The text the search index keeps for one issue.
#[derive(Debug, Clone, PartialEq)]
pub struct FtsDoc {
    pub rowid: i64,
    pub key: String,
    pub summary: String,
    pub description: String,
    pub comments: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mapped {
    pub issue: IssueRow,
    pub comments: Vec<CommentRow>,
    pub users: Vec<UserRow>,
    pub fts: FtsDoc,
}

fn text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
