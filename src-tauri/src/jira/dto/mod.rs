//! Jira REST API v3 answers, only the parts Taskboard reads.

mod attachment;
mod changelog;
mod comment;
mod issue;
mod meta;
mod remote_link;

pub use attachment::{Attachment, AttachmentAuthor};
pub(crate) use changelog::ChangelogPage;
pub use changelog::{ChangeItem, Changelog};
pub use comment::Comment;
pub(crate) use comment::{CommentPage, ParsedQueries};
pub use issue::{ApproximateCount, BulkFetch, Issue, SearchPage, parse_jira_time};
pub use meta::{
    Field, FieldSchema, Myself, Project, Status, StatusCategory, TenantInfo, User, Version,
};
pub use remote_link::{RemoteApplication, RemoteIcon, RemoteLink, RemoteObject};

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
