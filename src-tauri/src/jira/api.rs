//! The Jira endpoints Taskboard uses, one method each.

use serde_json::json;

use super::client::{BadRequest, JiraClient};
use super::dto::{
    ApproximateCount, BulkFetch, Comment, CommentPage, Field, Myself, ParsedQueries, Project,
    ProjectPage, Status, TenantInfo, Version,
};
use super::paging::{PAGE_SIZE, SearchPages, SearchRequest};
use crate::error::AppResult;

/// Jira's limit for `issue/bulkfetch`.
pub const BULK_FETCH_MAX: usize = 100;

impl JiraClient {
    pub async fn myself(&self) -> AppResult<Myself> {
        self.get("myself", &[]).await
    }

    /// The site's cloud id, which scoped tokens need; asked without credentials.
    pub async fn tenant_info(&self) -> AppResult<TenantInfo> {
        self.get_public(&self.site().tenant_info_url()).await
    }

    pub async fn fields(&self) -> AppResult<Vec<Field>> {
        self.get("field", &[]).await
    }

    pub async fn statuses(&self) -> AppResult<Vec<Status>> {
        self.get("status", &[]).await
    }

    pub async fn versions(&self, project_key: &str) -> AppResult<Vec<Version>> {
        self.get(&format!("project/{project_key}/versions"), &[])
            .await
    }

    /// Every project the user can browse, all pages.
    pub async fn projects(&self) -> AppResult<Vec<Project>> {
        let mut projects = Vec::new();
        loop {
            let start_at = projects.len().to_string();
            let page: ProjectPage = self
                .get(
                    "project/search",
                    &[("startAt", &start_at), ("maxResults", "50")],
                )
                .await?;
            let empty = page.values.is_empty();
            projects.extend(page.values);
            if page.is_last || empty {
                return Ok(projects);
            }
        }
    }

    /// The search's results, page by page.
    pub fn search_jql(&self, request: SearchRequest) -> SearchPages<'_> {
        SearchPages::new(self, request)
    }

    /// Up to `BULK_FETCH_MAX` issues by id or key; missing ones are left out of `issues`.
    pub async fn bulk_fetch(&self, ids: &[String], fields: &[String]) -> AppResult<BulkFetch> {
        let body = json!({ "issueIdsOrKeys": ids, "fields": fields });
        self.post("issue/bulkfetch", &[], &body, BadRequest::Plain)
            .await
    }

    pub async fn approximate_count(&self, jql: &str) -> AppResult<u64> {
        let body = json!({ "jql": jql });
        let answer: ApproximateCount = self
            .post("search/approximate-count", &[], &body, BadRequest::Jql)
            .await?;
        Ok(answer.count)
    }

    /// Every comment on an issue, oldest first, with Jira's rendered HTML.
    pub async fn comments(&self, issue_key: &str) -> AppResult<Vec<Comment>> {
        let path = format!("issue/{issue_key}/comment");
        let page_size = PAGE_SIZE.to_string();
        let mut comments = Vec::new();
        loop {
            let start_at = comments.len().to_string();
            let page: CommentPage = self
                .get(
                    &path,
                    &[
                        ("startAt", &start_at),
                        ("maxResults", &page_size),
                        ("orderBy", "created"),
                        ("expand", "renderedBody"),
                    ],
                )
                .await?;
            let empty = page.comments.is_empty();
            comments.extend(page.comments);
            if empty || comments.len() as u64 >= page.total {
                return Ok(comments);
            }
        }
    }

    /// Jira's problems with a query, strictly validated; empty when the query is valid.
    pub async fn check_jql(&self, jql: &str) -> AppResult<Vec<String>> {
        let body = json!({ "queries": [jql] });
        let parsed: ParsedQueries = self
            .post(
                "jql/parse",
                &[("validation", "strict")],
                &body,
                BadRequest::Jql,
            )
            .await?;
        Ok(parsed
            .queries
            .into_iter()
            .next()
            .map(|query| query.errors)
            .unwrap_or_default())
    }
}
