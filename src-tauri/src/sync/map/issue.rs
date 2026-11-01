//! The issue row, and the mapping of one issue to all of its rows.

use serde_json::Value;

use super::comments::comments_of;
use super::users::users_of;
use super::{FieldIds, FtsDoc, Mapped, text};
use crate::jira::dto::Issue;
use crate::sync::raw::{custom_values, kept_raw};
use crate::text::html_to_text;
use crate::time::utc;

#[derive(Debug, Clone, PartialEq)]
pub struct IssueRow {
    pub id: String,
    pub key: String,
    pub project_id: String,
    pub parent_id: Option<String>,
    /// JSON array of version ids.
    pub fix_versions: String,
    pub epic_color: Option<String>,
    pub summary: String,
    pub issue_type: String,
    pub type_is_subtask: bool,
    pub is_epic: bool,
    pub status_id: String,
    pub status_category: String,
    pub priority: Option<String>,
    pub assignee_id: Option<String>,
    pub reporter_id: Option<String>,
    pub sprint: Option<String>,
    pub sprint_id: Option<String>,
    pub sprint_state: Option<String>,
    /// JSON array.
    pub labels: String,
    pub due_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
    pub description_html: Option<String>,
    pub description_adf: Option<String>,
    pub comment_count: i64,
    /// Whether the search returned every comment, so the comment table is complete.
    pub comments_complete: bool,
    /// The custom fields' values (`raw::custom_values`).
    pub custom: Option<String>,
    /// What `issue_body.raw` keeps (`raw::kept_raw`).
    pub raw: String,
}

/// `None` when the issue lacks what every row needs (project, status, timestamps) or has a non-numeric id.
pub fn map_issue(issue: &Issue, ids: &FieldIds) -> Option<Mapped> {
    let f = &issue.fields;
    let rendered = &issue.rendered_fields;
    let issue_type = &f["issuetype"];
    let is_epic = issue_type["hierarchyLevel"].as_i64() == Some(1) || issue_type["name"] == "Epic";
    let (sprint, sprint_id, sprint_state) = sprint_of(ids.sprint.as_ref().map(|id| &f[id]));
    let comments = comments_of(issue);
    let description_html = text(&rendered["description"]).filter(|html| !html.is_empty());
    let row = IssueRow {
        id: issue.id.clone(),
        key: issue.key.clone(),
        project_id: text(&f["project"]["id"])?,
        parent_id: text(&f["parent"]["id"]),
        fix_versions: id_list(&f["fixVersions"]),
        epic_color: ids
            .epic_color
            .as_ref()
            .filter(|_| is_epic)
            .and_then(|id| text(&f[id])),
        summary: text(&f["summary"]).unwrap_or_default(),
        issue_type: text(&issue_type["name"]).unwrap_or_default(),
        type_is_subtask: issue_type["subtask"].as_bool().unwrap_or(false),
        is_epic,
        status_id: text(&f["status"]["id"])?,
        status_category: text(&f["status"]["statusCategory"]["key"])?,
        priority: text(&f["priority"]["name"]),
        assignee_id: text(&f["assignee"]["accountId"]),
        reporter_id: text(&f["reporter"]["accountId"]),
        sprint,
        sprint_id,
        sprint_state,
        labels: f["labels"]
            .as_array()
            .map_or_else(|| "[]".into(), |l| Value::from(l.clone()).to_string()),
        due_date: text(&f["duedate"]),
        created_at: utc(&f["created"])?,
        updated_at: utc(&f["updated"])?,
        resolved_at: utc(&f["resolutiondate"]),
        description_adf: f
            .get("description")
            .filter(|d| !d.is_null())
            .map(Value::to_string),
        comment_count: f["comment"]["total"]
            .as_i64()
            .unwrap_or(comments.len() as i64),
        comments_complete: f["comment"]["total"]
            .as_i64()
            .is_none_or(|total| total as usize <= comments.len()),
        custom: custom_values(f),
        raw: kept_raw(issue),
        description_html,
    };
    let fts = FtsDoc {
        rowid: issue.id.parse().ok()?,
        key: row.key.clone(),
        summary: row.summary.clone(),
        description: row
            .description_html
            .as_deref()
            .map(html_to_text)
            .unwrap_or_default(),
        comments: comments
            .iter()
            .map(|c| c.body_text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    };
    let users = users_of(issue);
    Some(Mapped {
        issue: row,
        comments,
        users,
        fts,
    })
}

fn id_list(value: &Value) -> String {
    let ids: Vec<Value> = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| text(&v["id"]).map(Value::from))
        .collect();
    Value::from(ids).to_string()
}

/// The active sprint, else the one with the highest id.
fn sprint_of(value: Option<&Value>) -> (Option<String>, Option<String>, Option<String>) {
    let sprints: Vec<&Value> = value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect();
    let chosen = sprints
        .iter()
        .find(|s| s["state"] == "active")
        .or_else(|| sprints.iter().max_by_key(|s| s["id"].as_i64().unwrap_or(0)));
    match chosen {
        Some(s) => (text(&s["name"]), text(&s["id"]), text(&s["state"])),
        None => (None, None, None),
    }
}
