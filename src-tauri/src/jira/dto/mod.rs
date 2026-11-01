//! Jira REST API v3 answers, only the parts Taskboard reads.

mod attachment;
mod changelog;
mod comment;
mod edit_meta;
mod issue;
mod meta;
mod permissions;
mod remote_link;
mod transition;

pub use attachment::{Attachment, AttachmentAuthor};
pub(crate) use changelog::ChangelogPage;
pub use changelog::{ChangeItem, Changelog};
pub use comment::Comment;
pub(crate) use comment::{CommentPage, ParsedQueries};
pub(crate) use edit_meta::EditMeta;
pub use issue::{ApproximateCount, BulkFetch, Issue, SearchPage, parse_jira_time};
pub use meta::{
    Field, FieldSchema, Myself, Project, Status, StatusCategory, TenantInfo, User, Version,
};
pub(crate) use permissions::MyPermissions;
pub use remote_link::{CreatedRemoteLink, RemoteApplication, RemoteIcon, RemoteLink, RemoteObject};
pub use transition::Transition;
pub(crate) use transition::Transitions;

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
