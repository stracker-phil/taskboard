//! Comments and the JQL check.

use serde::Deserialize;
use serde_json::Value;

use super::meta::User;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    #[serde(default)]
    pub author: Option<User>,
    /// ADF, kept for writes (M4).
    #[serde(default)]
    pub body: Value,
    /// Jira's HTML, untrusted until the UI sanitizes it.
    #[serde(default)]
    pub rendered_body: Option<String>,
    pub created: String,
    pub updated: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommentPage {
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub start_at: u64,
    #[serde(default)]
    pub total: u64,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct ParsedQueries {
    pub queries: Vec<ParsedQuery>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct ParsedQuery {
    #[serde(default)]
    pub errors: Vec<String>,
}
