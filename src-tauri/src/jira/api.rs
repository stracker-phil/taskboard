//! The Jira operations Taskboard uses, one method each, each one `Endpoint` of the gateway's allowlist.

use super::client::JiraClient;
use super::dto::{
    ApproximateCount, BulkFetch, Comment, CommentPage, Field, Myself, ParsedQueries, Project,
    ProjectPage, Status, TenantInfo, Version,
};
use super::gateway::{Endpoint, FieldIds, IssueRef, IssueRefs, Jql, ProjectKey};
use super::paging::{SearchPages, SearchRequest};
use crate::error::AppResult;

impl JiraClient {
    pub async fn myself(&self) -> AppResult<Myself> {
        self.gateway().send(Endpoint::Myself).await
    }

    /// The site's cloud id, which scoped tokens need; asked without credentials.
    pub async fn tenant_info(&self) -> AppResult<TenantInfo> {
        self.gateway().send(Endpoint::TenantInfo).await
    }

    pub async fn fields(&self) -> AppResult<Vec<Field>> {
        self.gateway().send(Endpoint::Fields).await
    }

    pub async fn statuses(&self) -> AppResult<Vec<Status>> {
        self.gateway().send(Endpoint::Statuses).await
    }

    pub async fn versions(&self, project_key: &str) -> AppResult<Vec<Version>> {
        let project = ProjectKey::parse(project_key)?;
        self.gateway()
            .send(Endpoint::ProjectVersions { project })
            .await
    }

    /// Every project the user can browse, all pages.
    pub async fn projects(&self) -> AppResult<Vec<Project>> {
        let mut projects = Vec::new();
        loop {
            let start_at = u32::try_from(projects.len()).unwrap_or(u32::MAX);
            let page: ProjectPage = self
                .gateway()
                .send(Endpoint::ProjectSearch { start_at })
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
        let endpoint = Endpoint::BulkFetch {
            issues: IssueRefs::parse(ids)?,
            fields: FieldIds::parse(fields)?,
        };
        self.gateway().send(endpoint).await
    }

    pub async fn approximate_count(&self, jql: &str) -> AppResult<u64> {
        let jql = Jql::parse(jql)?;
        let answer: ApproximateCount = self
            .gateway()
            .send(Endpoint::ApproximateCount { jql })
            .await?;
        Ok(answer.count)
    }

    /// Every comment on an issue, oldest first, with Jira's rendered HTML.
    pub async fn comments(&self, issue_key: &str) -> AppResult<Vec<Comment>> {
        let issue = IssueRef::parse(issue_key)?;
        let mut comments = Vec::new();
        loop {
            let start_at = u32::try_from(comments.len()).unwrap_or(u32::MAX);
            let page: CommentPage = self
                .gateway()
                .send(Endpoint::IssueComments {
                    issue: issue.clone(),
                    start_at,
                })
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
        let jql = Jql::parse(jql)?;
        let parsed: ParsedQueries = self.gateway().send(Endpoint::JqlParse { jql }).await?;
        Ok(parsed
            .queries
            .into_iter()
            .next()
            .map(|query| query.errors)
            .unwrap_or_default())
    }
}
