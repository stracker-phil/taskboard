//! One issue in full: what the peek and the Issue view show, and the detail sync of its comments, web links
//! and history.

mod activity;
mod attachments;
pub mod commands;
mod comments;
mod detail;
mod fetched;
mod history;
mod inline_image;
mod preview;
mod related;
mod web_links;

pub use activity::{ActivityItem, ChangeValue, issue_activity};
pub(crate) use activity::{status_names, user_names};
pub use attachments::{Attachment, attachments_of, is_on_site, keep_site_urls};
pub use comments::issue_sync_comments;
pub use detail::{CustomField, IssueDetail, issue_forget, issue_get};
pub use history::issue_sync_history;
pub use inline_image::{
    ImageMemory, MAX_INLINE_IMAGE_BYTES, attachment_image, data_url, declared_kind,
};
pub use preview::{
    attachment_file, cache_root, cached_path, clear as clear_attachment_cache, is_reusable,
    safe_filename,
};
pub use related::{Children, IssueLink, Related, RelatedIssue, issue_related};
pub use web_links::{WebLink, WebLinkKind, issue_sync_web_links, issue_web_links};
