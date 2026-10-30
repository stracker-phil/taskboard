//! The writes the fake Jira takes, shaped like Jira Cloud's: they change the served data and stamp `updated`.

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};

use super::http::Response;
use super::jira_time;
use super::routes::{Content, is_issue};
use crate::jira::dto::parse_jira_time;

/// `PUT /issue/{id}/assignee` with `{ "accountId": "…" }`, or `null` to unassign.
pub fn assign(site: &mut Content, id_or_key: &str, body: &Value) -> Response {
    let assignee = match assignee_for(site, body) {
        Ok(assignee) => assignee,
        Err(refused) => return refused,
    };
    let now = site.now;
    let Some(issue) = visible_issue_mut(site, id_or_key) else {
        return Response::json(
            404,
            json!({ "errorMessages": ["Issue does not exist or you do not have permission to see it."] }),
        );
    };
    issue["fields"]["assignee"] = assignee;
    stamp_after(issue, now);
    Response::raw(204, "text/plain", "")
}

/// The site's user object for the body's account id, `null` for none, or Jira's `400`.
fn assignee_for(site: &Content, body: &Value) -> Result<Value, Response> {
    let Some(account_id) = body["accountId"].as_str() else {
        return Ok(Value::Null);
    };
    site.data
        .users
        .iter()
        .find(|user| user["accountId"] == account_id)
        .cloned()
        .ok_or_else(|| {
            let reason = format!("User '{account_id}' cannot be assigned issues.");
            Response::json(
                400,
                json!({ "errorMessages": [], "errors": { "assignee": reason } }),
            )
        })
}

fn visible_issue_mut<'a>(site: &'a mut Content, id_or_key: &str) -> Option<&'a mut Value> {
    let hidden = &site.hidden;
    site.data.issues.iter_mut().find(|issue| {
        is_issue(issue, id_or_key) && !hidden.contains(issue["id"].as_str().unwrap_or_default())
    })
}

/// Stamps `updated` with the clock, or a second after the last update when the clock is behind it, so a write
/// always makes the issue newer, as on Jira.
fn stamp_after(issue: &mut Value, now: DateTime<Utc>) {
    let last = issue["fields"]["updated"]
        .as_str()
        .and_then(parse_jira_time)
        .map(|at| at.with_timezone(&Utc));
    let at = match last {
        Some(last) if last >= now => last + Duration::seconds(1),
        _ => now,
    };
    issue["fields"]["updated"] = json!(jira_time(at));
}
