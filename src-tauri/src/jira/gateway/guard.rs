//! The last check before the wire, kept apart from `endpoint` so a planning mistake can't widen what is
//! sent: a fixed table of the methods, paths and query keys Taskboard may use, and whether each one reads or
//! writes.

use reqwest::Method;

use super::endpoint::{Access, Planned};
use super::params::{is_issue_id, is_issue_key, is_project_key};
use crate::error::{AppError, AppResult};
use crate::jira::site::Site;
use Part::{AttachmentId, IssueIdOrKey, Literal, ProjectKey};
use Verb::{Get, Post, Put};

/// Whether this build may change Jira at all; the setting "Allow changes to Jira" then decides per user.
pub const WRITES_ENABLED: bool = true;

#[derive(Debug, Clone, Copy)]
enum Part {
    Literal(&'static str),
    ProjectKey,
    IssueIdOrKey,
    AttachmentId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verb {
    Get,
    Post,
    Put,
}

/// One allowed request under the REST API base (`…/rest/api/3/`).
struct Rule {
    verb: Verb,
    path: &'static [Part],
    query: &'static [&'static str],
    access: Access,
}

const fn read(verb: Verb, path: &'static [Part], query: &'static [&'static str]) -> Rule {
    Rule {
        verb,
        path,
        query,
        access: Access::Read,
    }
}

const fn write(verb: Verb, path: &'static [Part], query: &'static [&'static str]) -> Rule {
    Rule {
        verb,
        path,
        query,
        access: Access::Write,
    }
}

/// Each rule's token scope is listed in `docs/scopes.md`.
const RULES: &[Rule] = &[
    read(Get, &[Literal("myself")], &[]),
    read(Get, &[Literal("field")], &[]),
    read(Get, &[Literal("status")], &[]),
    read(
        Get,
        &[Literal("project"), Literal("search")],
        &["startAt", "maxResults"],
    ),
    read(
        Get,
        &[Literal("project"), ProjectKey, Literal("versions")],
        &[],
    ),
    read(
        Get,
        &[Literal("issue"), IssueIdOrKey, Literal("comment")],
        &["startAt", "maxResults", "orderBy", "expand"],
    ),
    read(
        Get,
        &[Literal("issue"), IssueIdOrKey, Literal("remotelink")],
        &[],
    ),
    read(
        Get,
        &[Literal("issue"), IssueIdOrKey, Literal("changelog")],
        &["startAt", "maxResults"],
    ),
    read(
        Get,
        &[Literal("attachment"), Literal("content"), AttachmentId],
        &["redirect"],
    ),
    read(Post, &[Literal("search"), Literal("jql")], &[]),
    read(
        Post,
        &[Literal("search"), Literal("approximate-count")],
        &[],
    ),
    read(Post, &[Literal("issue"), Literal("bulkfetch")], &[]),
    read(Post, &[Literal("jql"), Literal("parse")], &["validation"]),
    read(
        Get,
        &[Literal("issue"), IssueIdOrKey, Literal("transitions")],
        &[],
    ),
    read(
        Get,
        &[Literal("issue"), IssueIdOrKey, Literal("editmeta")],
        &[],
    ),
    write(Put, &[Literal("issue"), IssueIdOrKey], &[]),
    write(
        Put,
        &[Literal("issue"), IssueIdOrKey, Literal("assignee")],
        &[],
    ),
    write(
        Post,
        &[Literal("issue"), IssueIdOrKey, Literal("transitions")],
        &[],
    ),
    write(
        Post,
        &[Literal("issue"), IssueIdOrKey, Literal("comment")],
        &["expand"],
    ),
];

/// `Ok` only for a request in the table (or the unauthenticated tenant-info read) on this site.
pub fn check(planned: &Planned, site: &Site) -> AppResult<()> {
    if allowed(planned, site)? {
        Ok(())
    } else {
        Err(AppError::Internal(format!(
            "Taskboard doesn't call {} {}",
            planned.method,
            planned.url.path()
        )))
    }
}

fn allowed(planned: &Planned, site: &Site) -> AppResult<bool> {
    let url = &planned.url;
    let verb = match planned.method {
        Method::GET => Get,
        Method::POST => Post,
        Method::PUT => Put,
        _ => return Ok(false),
    };
    let plain = url.username().is_empty() && url.password().is_none() && url.fragment().is_none();
    let body_fits = verb != Get || planned.body.is_none();
    let access_open = planned.access == Access::Read || WRITES_ENABLED;
    if !plain || !body_fits || !access_open {
        return Ok(false);
    }
    if *url == site.tenant_info_url() {
        return Ok(verb == Get && !planned.authenticated && planned.access == Access::Read);
    }
    if !planned.authenticated {
        return Ok(false);
    }
    let base = site.api_base()?;
    if url.origin() != base.origin() {
        return Ok(false);
    }
    let Some(rest) = url.path().strip_prefix(base.path()) else {
        return Ok(false);
    };
    let segments: Vec<&str> = rest.split('/').collect();
    let keys: Vec<String> = url.query_pairs().map(|(key, _)| key.into_owned()).collect();
    Ok(RULES.iter().any(|rule| {
        rule.verb == verb
            && rule.access == planned.access
            && path_matches(rule.path, &segments)
            && query_matches(rule.query, &keys)
    }))
}

/// Compares the raw (still percent-encoded) segments, so an encoded `/`, `?` or `..` never matches.
fn path_matches(parts: &[Part], segments: &[&str]) -> bool {
    parts.len() == segments.len()
        && parts
            .iter()
            .zip(segments)
            .all(|(part, segment)| match part {
                Literal(text) => text == segment,
                ProjectKey => is_project_key(segment),
                IssueIdOrKey => is_issue_id(segment) || is_issue_key(segment),
                AttachmentId => is_issue_id(segment),
            })
}

/// Rejects duplicate keys and keys the rule does not list.
fn query_matches(allowed: &[&str], keys: &[String]) -> bool {
    keys.iter()
        .enumerate()
        .all(|(index, key)| allowed.contains(&key.as_str()) && !keys[..index].contains(key))
}
