//! The Jira operations Taskboard uses, one method each, each one `Endpoint` of the gateway's allowlist.

use super::client::JiraClient;
use super::dto::{
    ApproximateCount, BulkFetch, Changelog, ChangelogPage, Comment, CommentPage, Field, Issue,
    Myself, ParsedQueries, Project, ProjectPage, RemoteLink, SearchPage, Status, TenantInfo,
    Version,
};
use super::gateway::{Endpoint, FieldIds, IssueRef, IssueRefs, Jql, ProjectKey};
use super::paging::{SearchPages, SearchRequest};
use crate::error::AppResult;
use serde_json::Value;
use std::collections::BTreeSet;

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

    /// The issue's web links (remote links), in Jira's order.
    pub async fn remote_links(&self, issue_id: &str) -> AppResult<Vec<RemoteLink>> {
        let issue = IssueRef::parse(issue_id)?;
        self.gateway()
            .send(Endpoint::IssueRemoteLinks { issue })
            .await
    }

    /// Every change to an issue's fields, oldest first.
    pub async fn changelog(&self, issue_id: &str) -> AppResult<Vec<Changelog>> {
        let issue = IssueRef::parse(issue_id)?;
        let mut entries = Vec::new();
        loop {
            let start_at = u32::try_from(entries.len()).unwrap_or(u32::MAX);
            let page: ChangelogPage = self
                .gateway()
                .send(Endpoint::IssueChangelog {
                    issue: issue.clone(),
                    start_at,
                })
                .await?;
            let empty = page.values.is_empty();
            entries.extend(page.values);
            if page.is_last || empty || entries.len() as u64 >= page.total {
                return Ok(entries);
            }
        }
    }

    /// The custom fields the project's most recently updated issues have a value in.
    pub async fn used_custom_fields(&self, project_key: &str) -> AppResult<BTreeSet<String>> {
        let project = ProjectKey::parse(project_key)?;
        let page: SearchPage = self
            .gateway()
            .send(Endpoint::FieldUsage { project })
            .await?;
        Ok(filled_custom_fields(&page.issues))
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

/// Custom field ids with a value (not null, not an empty list or text) in any of the issues.
fn filled_custom_fields(issues: &[Issue]) -> BTreeSet<String> {
    issues
        .iter()
        .filter_map(|issue| issue.fields.as_object())
        .flatten()
        .filter(|(id, value)| id.starts_with("customfield_") && is_filled(value))
        .map(|(id, _)| id.clone())
        .collect()
}

/// A value worth showing: not null, and not an empty text, list or object.
pub fn is_filled(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => true,
    }
}
