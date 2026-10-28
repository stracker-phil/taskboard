//! Issue search, page by page. Jira's `nextPageToken` pages must be fetched one after another, so this is
//! a pull-based cursor rather than a parallel fetch.

use super::client::JiraClient;
use super::dto::SearchPage;
use super::gateway::{Endpoint, Expand, FieldIds, Jql, PageToken};
use crate::error::AppResult;

pub use super::gateway::PAGE_SIZE;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchRequest {
    pub jql: String,
    /// Always explicit; never `*all`.
    pub fields: Vec<String>,
    /// Comma-separated (`renderedFields`).
    pub expand: Option<String>,
}

pub struct SearchPages<'a> {
    client: &'a JiraClient,
    request: SearchRequest,
    next_token: Option<String>,
    done: bool,
}

impl<'a> SearchPages<'a> {
    pub(crate) fn new(client: &'a JiraClient, request: SearchRequest) -> Self {
        Self {
            client,
            request,
            next_token: None,
            done: false,
        }
    }

    /// The next page; `None` after the last page or after an error.
    pub async fn next_page(&mut self) -> Option<AppResult<SearchPage>> {
        if self.done {
            return None;
        }
        let answer = match self.endpoint() {
            Ok(endpoint) => self.client.gateway().send::<SearchPage>(endpoint).await,
            Err(refused) => Err(refused),
        };
        match &answer {
            Ok(page) if !page.is_final() => self.next_token = page.next_page_token.clone(),
            _ => self.done = true,
        }
        Some(answer)
    }

    fn endpoint(&self) -> AppResult<Endpoint> {
        Ok(Endpoint::SearchJql {
            jql: Jql::parse(&self.request.jql)?,
            fields: FieldIds::parse(&self.request.fields)?,
            expand: self
                .request
                .expand
                .as_deref()
                .map(Expand::parse)
                .transpose()?,
            page_token: self
                .next_token
                .as_deref()
                .map(PageToken::parse)
                .transpose()?,
        })
    }
}
