//! One issue in full: what the peek and the Issue view show, and the detail sync of its comments and web links.

mod activity;
pub mod commands;
mod comments;
mod detail;
mod related;
mod web_links;

pub use activity::{ActivityItem, ChangeValue, issue_activity};
pub(crate) use activity::{status_names, user_names};
pub use comments::issue_sync_comments;
pub use detail::{CustomField, IssueDetail, issue_forget, issue_get};
pub use related::{Children, IssueLink, Related, RelatedIssue, issue_related};
pub use web_links::{WebLink, WebLinkKind, issue_sync_web_links, issue_web_links};
