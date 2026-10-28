//! Builds Jira REST v3 issue and comment objects from the demo specs.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Value, json};

use super::catalog::{Kind, project, project_ref, sprint_json, version, version_json};
use super::clock::Clock;
use super::fields::{EPIC_COLOR, REVIEWER, SPRINT, STORY_POINTS};
use super::people::user;
use super::spec::{CommentSpec, IssueSpec};
use super::{SITE_URL, adf};

const FIRST_COMMENT_ID: u64 = 50_001;

/// The numeric Jira id of a demo key: the project's prefix digit followed by the issue number.
pub fn issue_id(key: &str) -> String {
    let (project_key, number) = key.split_once('-').expect("demo key");
    let number: u64 = number.parse().expect("demo key number");
    (project(project_key).id_prefix * 100_000 + number).to_string()
}

/// Issues (in table order) and comments by issue id.
pub fn render(
    specs: &[IssueSpec],
    comments: &[CommentSpec],
    clock: &Clock,
) -> (Vec<Value>, BTreeMap<String, Vec<Value>>) {
    let mut by_issue: HashMap<&str, Vec<(u64, &CommentSpec)>> = HashMap::new();
    let mut ordered: Vec<&CommentSpec> = comments.iter().collect();
    ordered.sort_by_key(|c| c.at);
    for (index, comment) in ordered.into_iter().enumerate() {
        by_issue
            .entry(comment.issue)
            .or_default()
            .push((FIRST_COMMENT_ID + index as u64, comment));
    }
    let specs_by_key: HashMap<&str, &IssueSpec> =
        specs.iter().map(|s| (s.key.as_str(), s)).collect();

    let mut issues = Vec::with_capacity(specs.len());
    let mut comment_map = BTreeMap::new();
    for spec in specs {
        let id = issue_id(&spec.key);
        let own = by_issue.remove(spec.key.as_str()).unwrap_or_default();
        let rendered: Vec<RenderedComment> = own
            .iter()
            .map(|(cid, c)| rendered_comment(&id, *cid, c, clock))
            .collect();
        let parent = spec.parent.as_deref().map(|key| {
            let parent = specs_by_key
                .get(key)
                .unwrap_or_else(|| panic!("demo parent {key}"));
            parent_json(parent)
        });
        let last_comment = own.iter().map(|(_, c)| c.at).max().unwrap_or("");
        let updated = spec.updated.as_str().max(last_comment);
        issues.push(issue_json(spec, &id, parent, updated, &rendered, clock));
        if !rendered.is_empty() {
            comment_map.insert(id, rendered.into_iter().map(|c| c.endpoint).collect());
        }
    }
    assert!(
        by_issue.is_empty(),
        "comments on unknown demo issues: {:?}",
        by_issue.keys()
    );
    (issues, comment_map)
}

struct RenderedComment {
    /// As embedded in `fields.comment`.
    field: Value,
    /// As embedded in `renderedFields.comment`.
    html: Value,
    /// As `GET /issue/{key}/comment?expand=renderedBody` returns it.
    endpoint: Value,
}

fn rendered_comment(issue_id: &str, id: u64, spec: &CommentSpec, clock: &Clock) -> RenderedComment {
    let at = clock.stamp(spec.at);
    let base = json!({
        "self": format!("{SITE_URL}/rest/api/3/issue/{issue_id}/comment/{id}"),
        "id": id.to_string(),
        "author": user(spec.author),
        "updateAuthor": user(spec.author),
        "created": at,
        "updated": at,
        "jsdPublic": true,
    });
    let with = |key: &str, value: Value| {
        let mut comment = base.clone();
        comment[key] = value;
        comment
    };
    let mut endpoint = with("body", adf::doc(spec.text));
    endpoint["renderedBody"] = json!(adf::html(spec.text));
    RenderedComment {
        field: with("body", adf::doc(spec.text)),
        html: with("body", json!(adf::html(spec.text))),
        endpoint,
    }
}

fn page(comments: Vec<Value>) -> Value {
    let total = comments.len();
    json!({ "comments": comments, "maxResults": total, "total": total, "startAt": 0 })
}

fn parent_json(parent: &IssueSpec) -> Value {
    let id = issue_id(&parent.key);
    json!({
        "id": id,
        "key": parent.key,
        "self": format!("{SITE_URL}/rest/api/3/issue/{id}"),
        "fields": {
            "summary": parent.summary,
            "status": parent.status.json(),
            "priority": parent.priority.json(),
            "issuetype": parent.kind.json(),
        },
    })
}

fn issue_json(
    spec: &IssueSpec,
    id: &str,
    parent: Option<Value>,
    updated: &str,
    comments: &[RenderedComment],
    clock: &Clock,
) -> Value {
    let project_key = spec.key.split_once('-').expect("demo key").0;
    let fix_versions: Vec<Value> = spec
        .fix
        .map(|name| version_json(version(project_key, name), clock))
        .into_iter()
        .collect();
    let sprints: Vec<Value> = spec
        .sprints
        .iter()
        .map(|&s| sprint_json(s, clock))
        .collect();
    let mut fields = json!({
        "summary": spec.summary,
        "status": spec.status.json(),
        "issuetype": spec.kind.json(),
        "priority": spec.priority.json(),
        "assignee": spec.assignee.map(user),
        "reporter": user(spec.reporter),
        "labels": spec.labels,
        "duedate": spec.due.as_deref().map(|d| clock.date(d)),
        "fixVersions": fix_versions,
        "created": clock.stamp(&spec.created),
        "updated": clock.stamp(updated),
        "resolutiondate": spec.status.is_done().then(|| clock.stamp(updated)),
        "project": project_ref(project(project_key)),
        "comment": page(comments.iter().map(|c| c.field.clone()).collect()),
        "description": spec.description.as_deref().map(adf::doc),
        SPRINT: (!sprints.is_empty()).then_some(sprints),
        STORY_POINTS: spec.points,
        REVIEWER: spec.reviewer.map(user),
        EPIC_COLOR: if spec.kind == Kind::Epic { spec.epic_color } else { None },
    });
    if let Some(parent) = parent {
        fields["parent"] = parent;
    }
    json!({
        "expand": "renderedFields,names,schema,operations,editmeta,changelog,versionedRepresentations",
        "id": id,
        "self": format!("{SITE_URL}/rest/api/3/issue/{id}"),
        "key": spec.key,
        "fields": fields,
        "renderedFields": {
            "description": spec.description.as_deref().map(adf::html),
            "comment": page(comments.iter().map(|c| c.html.clone()).collect()),
        },
    })
}
