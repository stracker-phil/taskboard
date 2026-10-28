//! Issue search, page by page. Jira's `nextPageToken` pages must be fetched one after another, so this is
//! a pull-based cursor rather than a parallel fetch.

use serde_json::{Value, json};

use super::client::{BadRequest, JiraClient};
use super::dto::SearchPage;
use crate::error::AppResult;

pub const PAGE_SIZE: u32 = 100;

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
        let answer = self
            .client
            .post::<SearchPage>("search/jql", &[], &self.body(), BadRequest::Jql)
            .await;
        match &answer {
            Ok(page) if !page.is_final() => self.next_token = page.next_page_token.clone(),
            _ => self.done = true,
        }
        Some(answer)
    }

    fn body(&self) -> Value {
        let mut body = json!({
            "jql": self.request.jql,
            "maxResults": PAGE_SIZE,
            "fields": self.request.fields,
        });
        if let Some(expand) = &self.request.expand {
            body["expand"] = json!(expand);
        }
        if let Some(token) = &self.next_token {
            body["nextPageToken"] = json!(token);
        }
        body
    }
}
