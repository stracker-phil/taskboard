//! Jira REST API v3 answers, only the parts Taskboard reads.

mod comment;
mod issue;
mod meta;

pub use comment::Comment;
pub(crate) use comment::{CommentPage, ParsedQueries};
pub use issue::{ApproximateCount, BulkFetch, Issue, SearchPage, parse_jira_time};
pub use meta::{
    Field, FieldSchema, Myself, Project, Status, StatusCategory, TenantInfo, User, Version,
};

use serde::Deserialize;

/// `project/search` pages by offset.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectPage {
    #[serde(default)]
    pub values: Vec<Project>,
    #[serde(default)]
    pub is_last: bool,
}
