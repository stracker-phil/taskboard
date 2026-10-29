//! The allowlist: every operation Taskboard may ask of Jira, and the exact request each one becomes.

use reqwest::Method;
use serde_json::{Value, json};
use url::Url;

use super::params::{
    AttachmentId, Expand, FieldIds, IssueRef, IssueRefs, Jql, PageToken, ProjectKey,
};
use crate::error::{AppError, AppResult};
use crate::jira::site::Site;

/// Results per search, comment or changelog page.
pub const PAGE_SIZE: u32 = 100;
const PROJECT_PAGE_SIZE: u32 = 50;

/// What a field-usage sample reads: every field but the large ones no custom field lives in.
const USAGE_FIELDS: &[&str] = &[
    "*all",
    "-description",
    "-comment",
    "-attachment",
    "-worklog",
    "-issuelinks",
    "-subtasks",
    "-environment",
];

/// Reads may be retried; writes never are, and stay refused until writes are enabled (`guard`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

/// Whether a `400` means Jira rejected the request's JQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadRequest {
    Plain,
    Jql,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Endpoint {
    Myself,
    /// The site's cloud id, asked without credentials.
    TenantInfo,
    Fields,
    Statuses,
    ProjectSearch {
        start_at: u32,
    },
    ProjectVersions {
        project: ProjectKey,
    },
    SearchJql {
        jql: Jql,
        fields: FieldIds,
        expand: Option<Expand>,
        page_token: Option<PageToken>,
    },
    BulkFetch {
        issues: IssueRefs,
        fields: FieldIds,
    },
    ApproximateCount {
        jql: Jql,
    },
    IssueComments {
        issue: IssueRef,
        start_at: u32,
    },
    /// The issue's web links (Jira's remote links), all in one answer.
    IssueRemoteLinks {
        issue: IssueRef,
    },
    /// A page of the issue's changelog, oldest first.
    IssueChangelog {
        issue: IssueRef,
        start_at: u32,
    },
    /// An attachment's bytes, served by the site itself rather than redirected to Atlassian's media host.
    AttachmentContent {
        attachment: AttachmentId,
    },
    /// Strict validation of a query.
    JqlParse {
        jql: Jql,
    },
    /// The project's most recently updated issues with all their fields, to see which custom fields it fills.
    FieldUsage {
        project: ProjectKey,
    },
}

/// A request ready for the guard and the wire.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub method: Method,
    pub url: Url,
    pub body: Option<Value>,
    pub authenticated: bool,
    pub access: Access,
    pub bad_request: BadRequest,
}

impl Endpoint {
    pub fn plan(&self, site: &Site) -> AppResult<Planned> {
        match self {
            Self::Myself => Rest::get(&["myself"]).plan(site),
            Self::TenantInfo => Ok(Planned {
                method: Method::GET,
                url: site.tenant_info_url(),
                body: None,
                authenticated: false,
                access: Access::Read,
                bad_request: BadRequest::Plain,
            }),
            Self::Fields => Rest::get(&["field"]).plan(site),
            Self::Statuses => Rest::get(&["status"]).plan(site),
            Self::ProjectSearch { start_at } => Rest::get(&["project", "search"])
                .query("startAt", start_at)
                .query("maxResults", PROJECT_PAGE_SIZE)
                .plan(site),
            Self::ProjectVersions { project } => {
                Rest::get(&["project", project.as_str(), "versions"]).plan(site)
            }
            Self::SearchJql {
                jql,
                fields,
                expand,
                page_token,
            } => Rest::read_post(
                &["search", "jql"],
                search_body(jql, fields, expand.as_ref(), page_token.as_ref()),
            )
            .jql_errors()
            .plan(site),
            Self::BulkFetch { issues, fields } => Rest::read_post(
                &["issue", "bulkfetch"],
                json!({ "issueIdsOrKeys": issues.as_strs(), "fields": fields.as_slice() }),
            )
            .plan(site),
            Self::ApproximateCount { jql } => Rest::read_post(
                &["search", "approximate-count"],
                json!({ "jql": jql.as_str() }),
            )
            .jql_errors()
            .plan(site),
            Self::IssueComments { issue, start_at } => {
                Rest::get(&["issue", issue.as_str(), "comment"])
                    .query("startAt", start_at)
                    .query("maxResults", PAGE_SIZE)
                    .query("orderBy", "created")
                    .query("expand", "renderedBody")
                    .plan(site)
            }
            Self::IssueRemoteLinks { issue } => {
                Rest::get(&["issue", issue.as_str(), "remotelink"]).plan(site)
            }
            Self::IssueChangelog { issue, start_at } => {
                Rest::get(&["issue", issue.as_str(), "changelog"])
                    .query("startAt", start_at)
                    .query("maxResults", PAGE_SIZE)
                    .plan(site)
            }
            Self::AttachmentContent { attachment } => {
                Rest::get(&["attachment", "content", attachment.as_str()])
                    .query("redirect", "false")
                    .plan(site)
            }
            Self::FieldUsage { project } => Rest::read_post(
                &["search", "jql"],
                json!({
                    "jql": format!("project = \"{}\" ORDER BY updated DESC", project.as_str()),
                    "maxResults": PAGE_SIZE,
                    "fields": USAGE_FIELDS,
                }),
            )
            .jql_errors()
            .plan(site),
            Self::JqlParse { jql } => {
                Rest::read_post(&["jql", "parse"], json!({ "queries": [jql.as_str()] }))
                    .query("validation", "strict")
                    .jql_errors()
                    .plan(site)
            }
        }
    }
}

fn search_body(
    jql: &Jql,
    fields: &FieldIds,
    expand: Option<&Expand>,
    page_token: Option<&PageToken>,
) -> Value {
    let mut body = json!({
        "jql": jql.as_str(),
        "maxResults": PAGE_SIZE,
        "fields": fields.as_slice(),
    });
    if let Some(expand) = expand {
        body["expand"] = json!(expand.as_str());
    }
    if let Some(token) = page_token {
        body["nextPageToken"] = json!(token.as_str());
    }
    body
}

/// A request under the REST API base, authenticated.
struct Rest<'a> {
    method: Method,
    segments: &'a [&'a str],
    query: Vec<(&'static str, String)>,
    body: Option<Value>,
    bad_request: BadRequest,
}

impl<'a> Rest<'a> {
    fn get(segments: &'a [&'a str]) -> Self {
        Self {
            method: Method::GET,
            segments,
            query: Vec::new(),
            body: None,
            bad_request: BadRequest::Plain,
        }
    }

    /// A read that Jira takes as a POST (searches, counts, bulk fetches).
    fn read_post(segments: &'a [&'a str], body: Value) -> Self {
        Self {
            method: Method::POST,
            body: Some(body),
            ..Self::get(segments)
        }
    }

    fn query(mut self, key: &'static str, value: impl ToString) -> Self {
        self.query.push((key, value.to_string()));
        self
    }

    fn jql_errors(mut self) -> Self {
        self.bad_request = BadRequest::Jql;
        self
    }

    fn plan(self, site: &Site) -> AppResult<Planned> {
        let mut url = site.api_base()?;
        // Each segment is percent-encoded on its own, so a value can't add segments, a query or a fragment.
        url.path_segments_mut()
            .map_err(|()| AppError::Internal("The Jira API base can't take a path".into()))?
            .pop_if_empty()
            .extend(self.segments);
        if !self.query.is_empty() {
            url.query_pairs_mut().extend_pairs(&self.query);
        }
        Ok(Planned {
            method: self.method,
            url,
            body: self.body,
            authenticated: true,
            access: Access::Read,
            bad_request: self.bad_request,
        })
    }
}
