//! An issue's comments as rows: Jira's HTML, the ADF body and plain text for search.

use serde_json::Value;

use super::text;
use crate::jira::dto::Issue;
use crate::text::html_to_text;
use crate::time::utc;

#[derive(Debug, Clone, PartialEq)]
pub struct CommentRow {
    pub id: String,
    pub issue_id: String,
    pub author_id: Option<String>,
    pub body_html: Option<String>,
    pub body_adf: Option<String>,
    pub body_text: String,
    pub created_at: String,
    pub updated_at: String,
}

pub(super) fn comments_of(issue: &Issue) -> Vec<CommentRow> {
    let rendered = issue.rendered_fields["comment"]["comments"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let html_of = |id: &str| {
        rendered
            .iter()
            .find(|c| c["id"] == id)
            .and_then(|c| text(&c["body"]))
    };
    issue.fields["comment"]["comments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let id = text(&c["id"])?;
            let body_html = html_of(&id);
            Some(CommentRow {
                body_text: body_html.as_deref().map(html_to_text).unwrap_or_default(),
                body_html,
                body_adf: Some(&c["body"])
                    .filter(|b| b.is_object())
                    .map(Value::to_string),
                issue_id: issue.id.clone(),
                author_id: text(&c["author"]["accountId"]),
                created_at: utc(&c["created"])?,
                updated_at: utc(&c["updated"])?,
                id,
            })
        })
        .collect()
}
