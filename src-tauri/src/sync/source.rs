//! What the sync asks of an issue tracker (ADR 0029). Jira (`jira_source`) is the only source today; another
//! one (GitHub) is a second implementation, chosen in `SyncContext::source`.
//!
//! The answers keep Jira's shapes (`jira::dto`), which `map` turns into cache rows.

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use crate::error::AppResult;
use crate::jira::SearchRequest;
use crate::jira::dto::{
    BulkFetch, Comment, Field, Myself, Project, RemoteLink, SearchPage, Status, Version,
};

/// A source's answer, boxed so a source can sit behind `dyn Source`.
pub type Answer<'a, T> = Pin<Box<dyn Future<Output = AppResult<T>> + Send + 'a>>;

/// Which tracker a cached row comes from: the `source` column of `cache.issue`, `cache.space` and
/// `cache.comment` (and `cache.remote_link`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Jira,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jira => "jira",
        }
    }
}

/// A query's issues, one page after another.
pub trait IssuePages: Send {
    /// The next page; `None` after the last page or after an error.
    fn next_page(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Option<AppResult<SearchPage>>> + Send + '_>>;
}

pub trait Source: Send + Sync {
    fn kind(&self) -> SourceKind;
    /// The tracker's fields, custom ones included.
    fn fields(&self) -> Answer<'_, Vec<Field>>;
    fn statuses(&self) -> Answer<'_, Vec<Status>>;
    /// Every space (project, repository) the user can see.
    fn spaces(&self) -> Answer<'_, Vec<Project>>;
    /// A space's releases, by its key.
    fn versions<'a>(&'a self, space_key: &'a str) -> Answer<'a, Vec<Version>>;
    /// Who the user is.
    fn myself(&self) -> Answer<'_, Myself>;
    /// About how many issues a query finds.
    fn count<'a>(&'a self, query: &'a str) -> Answer<'a, u64>;
    fn issues(&self, request: SearchRequest) -> Box<dyn IssuePages + '_>;
    /// Issues by id; missing ones are left out.
    fn fetch<'a>(&'a self, ids: &'a [String], fields: &'a [String]) -> Answer<'a, BulkFetch>;
    /// Every comment on an issue, oldest first.
    fn comments<'a>(&'a self, issue_key: &'a str) -> Answer<'a, Vec<Comment>>;
    /// An issue's web links (pull requests, documents), by its id.
    fn remote_links<'a>(&'a self, issue_id: &'a str) -> Answer<'a, Vec<RemoteLink>>;
    /// The custom fields a space's recent issues fill.
    fn used_custom_fields<'a>(&'a self, space_key: &'a str) -> Answer<'a, BTreeSet<String>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jira_is_stored_as_the_text_jira() {
        assert_eq!(SourceKind::Jira.as_str(), "jira");
    }
}
