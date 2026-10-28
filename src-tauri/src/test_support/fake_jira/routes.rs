//! The endpoints the fake Jira answers, shaped like Jira Cloud's REST API v3.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use super::http::{Recorded, Response};
use super::jql::{self, Context};
use crate::jira::dto::parse_jira_time;
use crate::test_support::demo::JiraData;

const DEFAULT_PAGE_SIZE: usize = 50;

/// The site's content plus what the knobs hide.
pub struct Content {
    pub data: JiraData,
    pub now: DateTime<Utc>,
    /// Revoked: the user can no longer see these issue ids.
    pub hidden: HashSet<String>,
    pub page_size: Option<usize>,
    pub max_results_cap: usize,
}

pub fn route(site: &Content, request: &Recorded) -> Response {
    if request.path == "/_edge/tenant_info" {
        return Response::json(200, json!({ "cloudId": site.data.cloud_id }));
    }
    let Some(path) = api_path(&request.path, &site.data.cloud_id) else {
        return not_found();
    };
    if request.authorization.is_none() {
        return Response::json(401, json!({ "errorMessages": ["Authentication required"] }));
    }
    let parts: Vec<&str> = path.split('/').collect();
    match (request.method.as_str(), parts.as_slice()) {
        ("GET", ["myself"]) => Response::json(200, site.data.myself.clone()),
        ("GET", ["field"]) => Response::json(200, json!(site.data.fields)),
        ("GET", ["status"]) => Response::json(200, json!(site.data.statuses)),
        ("GET", ["project", "search"]) => projects(site, request),
        ("GET", ["project", key, "versions"]) => match site.data.versions.get(*key) {
            Some(versions) => Response::json(200, json!(versions)),
            None if has_project(site, key) => Response::json(200, json!([])),
            None => not_found(),
        },
        ("POST", ["search", "jql"]) => search(site, &request.body),
        ("POST", ["search", "approximate-count"]) => {
            let count = matching(site, request.body["jql"].as_str().unwrap_or("")).len();
            Response::json(200, json!({ "count": count }))
        }
        ("POST", ["issue", "bulkfetch"]) => bulk_fetch(site, &request.body),
        ("GET", ["issue", key, "comment"]) => comments(site, key, request),
        ("POST", ["jql", "parse"]) => parse_jql(&request.body),
        _ => not_found(),
    }
}

/// The REST path for classic (`/rest/api/3/…`) and scoped (`/ex/jira/<cloudId>/rest/api/3/…`) calls.
fn api_path<'a>(path: &'a str, cloud_id: &str) -> Option<&'a str> {
    path.strip_prefix("/rest/api/3/").or_else(|| {
        path.strip_prefix("/ex/jira/")?
            .strip_prefix(cloud_id)?
            .strip_prefix("/rest/api/3/")
    })
}

fn not_found() -> Response {
    Response::json(404, json!({ "errorMessages": ["Not found"] }))
}

fn has_project(site: &Content, key: &str) -> bool {
    site.data.projects.iter().any(|p| p["key"] == key)
}

fn offset_query(request: &Recorded, default_size: usize) -> (usize, usize) {
    let number = |name: &str, default: usize| {
        request
            .query_value(name)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    (
        number("startAt", 0),
        number("maxResults", default_size).max(1),
    )
}

fn projects(site: &Content, request: &Recorded) -> Response {
    let (start, size) = offset_query(request, DEFAULT_PAGE_SIZE);
    let all = &site.data.projects;
    let page: Vec<&Value> = all.iter().skip(start).take(size).collect();
    Response::json(
        200,
        json!({
            "values": page,
            "startAt": start,
            "maxResults": size,
            "total": all.len(),
            "isLast": start + size >= all.len(),
        }),
    )
}

/// Visible issues matching `jql`, oldest change first.
fn matching<'a>(site: &'a Content, jql: &str) -> Vec<&'a Value> {
    let zone = site.data.myself["timeZone"]
        .as_str()
        .and_then(|z| z.parse().ok())
        .unwrap_or(chrono_tz::UTC);
    let context = Context {
        now: site.now,
        zone,
    };
    let query = jql::parse_or_all(jql);
    let mut found: Vec<&Value> = site
        .data
        .issues
        .iter()
        .filter(|issue| {
            !site
                .hidden
                .contains(issue["id"].as_str().unwrap_or_default())
        })
        .filter(|issue| jql::matches(&query, issue, &context))
        .collect();
    found.sort_by_key(|issue| {
        let updated = issue["fields"]["updated"]
            .as_str()
            .and_then(parse_jira_time);
        (
            updated,
            issue["id"].as_str().and_then(|id| id.parse::<u64>().ok()),
        )
    });
    found
}

fn search(site: &Content, body: &Value) -> Response {
    let found = matching(site, body["jql"].as_str().unwrap_or(""));
    let asked = body["maxResults"]
        .as_u64()
        .map_or(DEFAULT_PAGE_SIZE, |n| n as usize);
    let size = site
        .page_size
        .unwrap_or(asked.min(site.max_results_cap))
        .max(1);
    let start = body["nextPageToken"]
        .as_str()
        .and_then(|token| token.strip_prefix("page-")?.parse().ok())
        .unwrap_or(0);
    let end = (start + size).min(found.len());
    let is_last = end >= found.len();
    let issues: Vec<Value> = found
        .get(start..end)
        .unwrap_or_default()
        .iter()
        .map(|issue| shaped(issue, &body["fields"], &body["expand"]))
        .collect();
    let mut answer = json!({ "issues": issues, "isLast": is_last });
    if !is_last {
        answer["nextPageToken"] = json!(format!("page-{end}"));
    }
    Response::json(200, answer)
}

/// The issue with only the asked fields; `renderedFields` only when expanded.
fn shaped(issue: &Value, fields: &Value, expand: &Value) -> Value {
    let asked = string_list(fields);
    let all = asked.is_empty() || asked.contains(&"*all");
    let mut shaped = json!({ "id": issue["id"], "key": issue["key"], "self": issue["self"] });
    let kept = issue["fields"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(name, _)| all || asked.contains(&name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()));
    shaped["fields"] = Value::Object(kept.collect());
    if expand
        .as_str()
        .unwrap_or_default()
        .contains("renderedFields")
    {
        shaped["renderedFields"] = issue["renderedFields"].clone();
    }
    shaped
}

fn find_issue<'a>(site: &'a Content, id_or_key: &str) -> Option<&'a Value> {
    site.data.issues.iter().find(|issue| {
        is_issue(issue, id_or_key)
            && !site
                .hidden
                .contains(issue["id"].as_str().unwrap_or_default())
    })
}

pub fn is_issue(issue: &Value, id_or_key: &str) -> bool {
    issue["id"] == id_or_key || issue["key"] == id_or_key
}

fn string_list(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn bulk_fetch(site: &Content, body: &Value) -> Response {
    let asked = string_list(&body["issueIdsOrKeys"]);
    let mut issues = Vec::new();
    let mut errors = Vec::new();
    for id in asked {
        match find_issue(site, id) {
            Some(issue) => issues.push(shaped(issue, &body["fields"], &body["expand"])),
            None => errors.push(json!({
                "issueIdsOrKeys": [id],
                "errorMessages": ["The issue does not exist or you do not have permission to see it."],
                "status": 404,
            })),
        }
    }
    Response::json(200, json!({ "issues": issues, "issueErrors": errors }))
}

fn comments(site: &Content, id_or_key: &str, request: &Recorded) -> Response {
    let Some(issue) = find_issue(site, id_or_key) else {
        return not_found();
    };
    let id = issue["id"].as_str().unwrap_or_default();
    let all = site.data.comments.get(id).cloned().unwrap_or_default();
    let (start, size) = offset_query(request, DEFAULT_PAGE_SIZE);
    let rendered = request
        .query_value("expand")
        .is_some_and(|e| e.contains("renderedBody"));
    let page: Vec<Value> = all
        .iter()
        .skip(start)
        .take(size)
        .map(|comment| {
            let mut comment = comment.clone();
            if !rendered && let Some(fields) = comment.as_object_mut() {
                fields.remove("renderedBody");
            }
            comment
        })
        .collect();
    Response::json(
        200,
        json!({ "comments": page, "startAt": start, "maxResults": size, "total": all.len() }),
    )
}

fn parse_jql(body: &Value) -> Response {
    let queries: Vec<Value> = string_list(&body["queries"])
        .into_iter()
        .map(|query| {
            let errors: Vec<String> = match jql::parse(query) {
                Some(_) => Vec::new(),
                None => vec![format!(
                    "Error in the JQL Query: '{query}' could not be parsed."
                )],
            };
            json!({ "query": query, "errors": errors })
        })
        .collect();
    Response::json(200, json!({ "queries": queries }))
}
