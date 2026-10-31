//! The writes the fake Jira takes, shaped like Jira Cloud's: they change the served data and stamp `updated`.

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};

use super::http::Response;
use super::routes::{Content, is_issue};
use super::{jira_time, push_comment};
use crate::jira::dto::parse_jira_time;

/// `PUT /issue/{id}/assignee` with `{ "accountId": "…" }`, or `null` to unassign.
pub fn assign(site: &mut Content, id_or_key: &str, body: &Value) -> Response {
    let assignee = match assignee_for(site, body) {
        Ok(assignee) => assignee,
        Err(refused) => return refused,
    };
    let now = site.now;
    let Some(issue) = visible_issue_mut(site, id_or_key) else {
        return not_found();
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

/// `POST /issue/{id}/remotelink` with `{ "object": { "url", "title" } }`: the link joins the issue's web links,
/// answered `201` with its id like Jira; an address or title missing is Jira's `400`.
pub fn add_remote_link(site: &mut Content, id_or_key: &str, body: &Value) -> Response {
    let object = &body["object"];
    let (Some(url), Some(title)) = (object["url"].as_str(), object["title"].as_str()) else {
        return Response::json(
            400,
            json!({ "errorMessages": [], "errors": { "object": "A url and a title are required." } }),
        );
    };
    let Some(id) =
        visible_issue(site, id_or_key).and_then(|issue| issue["id"].as_str().map(str::to_string))
    else {
        return not_found();
    };
    let next = 80_000 + site.data.remote_links.values().map(Vec::len).sum::<usize>() as u64 + 1;
    site.data
        .remote_links
        .entry(id)
        .or_default()
        .push(json!({ "id": next, "object": { "url": url, "title": title } }));
    Response::json(
        201,
        json!({ "id": next, "self": format!("{}/rest/api/3/issue/{id_or_key}/remotelink/{next}", crate::test_support::demo::SITE_URL) }),
    )
}

/// The priorities every demo issue may take, Jira's defaults.
const PRIORITIES: [(&str, &str); 5] = [
    ("1", "Highest"),
    ("2", "High"),
    ("3", "Medium"),
    ("4", "Low"),
    ("5", "Lowest"),
];

/// `GET /issue/{id}/editmeta`: the edit screen offers the priority with Jira's five defaults.
pub fn edit_meta(site: &Content, id_or_key: &str) -> Response {
    if visible_issue(site, id_or_key).is_none() {
        return not_found();
    }
    let allowed: Vec<Value> = PRIORITIES
        .iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect();
    Response::json(
        200,
        json!({ "fields": { "priority": { "name": "Priority", "allowedValues": allowed } } }),
    )
}

/// `PUT /issue/{id}` with `{ "fields": { "priority": { "name": "…" } } }`; other fields are refused like a field
/// missing from the edit screen.
pub fn edit(site: &mut Content, id_or_key: &str, body: &Value) -> Response {
    let fields = body["fields"].as_object().cloned().unwrap_or_default();
    let asked = fields.get("priority").and_then(|p| p["name"].as_str());
    let priority = asked.and_then(|name| PRIORITIES.iter().find(|(_, n)| *n == name));
    let (Some((id, name)), 1) = (priority, fields.len()) else {
        return Response::json(
            400,
            json!({ "errorMessages": [], "errors": { "priority": "Field cannot be set. It is not on the appropriate screen, or unknown." } }),
        );
    };
    let now = site.now;
    let Some(issue) = visible_issue_mut(site, id_or_key) else {
        return not_found();
    };
    issue["fields"]["priority"] = json!({ "id": id, "name": name });
    stamp_after(issue, now);
    Response::raw(204, "text/plain", "")
}

/// `GET /issue/{id}/transitions`: one move to every other status, named like it; the move's id is the status's.
pub fn transitions(site: &Content, id_or_key: &str) -> Response {
    let Some(issue) = visible_issue(site, id_or_key) else {
        return not_found();
    };
    let current = &issue["fields"]["status"]["id"];
    let moves: Vec<Value> = site
        .data
        .statuses
        .iter()
        .filter(|status| status["id"] != *current)
        .map(|status| {
            json!({
                "id": status["id"],
                "name": status["name"],
                "to": status,
                "hasScreen": site.screens.contains(status["id"].as_str().unwrap_or_default()),
                "isAvailable": true,
            })
        })
        .collect();
    Response::json(200, json!({ "transitions": moves }))
}

/// `POST /issue/{id}/transitions` with `{ "transition": { "id": "…" } }`.
pub fn transition(site: &mut Content, id_or_key: &str, body: &Value) -> Response {
    let asked = body["transition"]["id"].as_str().unwrap_or_default();
    let Some(target) = site
        .data
        .statuses
        .iter()
        .find(|s| s["id"] == asked)
        .cloned()
    else {
        return Response::json(
            400,
            json!({ "errorMessages": [format!("Transition id '{asked}' is not valid for this issue.")] }),
        );
    };
    let now = site.now;
    let Some(issue) = visible_issue_mut(site, id_or_key) else {
        return not_found();
    };
    if issue["fields"]["status"]["id"] == target["id"] {
        return Response::json(
            400,
            json!({ "errorMessages": [format!("Transition id '{asked}' is not valid for this issue.")] }),
        );
    }
    issue["fields"]["status"] = target;
    stamp_after(issue, now);
    Response::raw(204, "text/plain", "")
}

/// `POST /issue/{id}/comment` with `{ "body": <ADF> }`: the comment by `myself`, answered `201` like Jira, with
/// `renderedBody` (paragraphs of the body's text).
pub fn comment(site: &mut Content, id_or_key: &str, body: &Value) -> Response {
    let adf = &body["body"];
    if adf["type"] != "doc" {
        return Response::json(
            400,
            json!({ "errorMessages": [], "errors": { "comment": "Comment body can not be empty!" } }),
        );
    }
    let next_id = site.data.comments.values().flatten().count() + 900_000;
    let (now, author) = (site.now, site.data.myself.clone());
    let Some(issue) = visible_issue_mut(site, id_or_key) else {
        return not_found();
    };
    let stamp = jira_time(now);
    let rendered = rendered_paragraphs(adf);
    let comment = json!({
        "id": next_id.to_string(),
        "author": author,
        "body": adf,
        "renderedBody": rendered,
        "created": stamp,
        "updated": stamp,
    });
    push_comment(&mut issue["fields"]["comment"], comment.clone());
    let mut shown = comment.clone();
    shown["body"] = json!(rendered);
    push_comment(&mut issue["renderedFields"]["comment"], shown);
    stamp_after(issue, now);
    let id = issue["id"].as_str().unwrap_or_default().to_string();
    site.data
        .comments
        .entry(id)
        .or_default()
        .push(comment.clone());
    Response::json(201, comment)
}

/// `<p>` per paragraph with its text, the way Jira renders a plain comment.
fn rendered_paragraphs(adf: &Value) -> String {
    let paragraphs = adf["content"].as_array().into_iter().flatten();
    paragraphs
        .map(|p| {
            let texts = p["content"].as_array().into_iter().flatten();
            let text: String = texts
                .map(|node| {
                    node["text"]
                        .as_str()
                        .unwrap_or(if node["type"] == "hardBreak" {
                            "\n"
                        } else {
                            ""
                        })
                })
                .collect();
            format!("<p>{text}</p>")
        })
        .collect()
}

fn not_found() -> Response {
    Response::json(
        404,
        json!({ "errorMessages": ["Issue does not exist or you do not have permission to see it."] }),
    )
}

fn visible_issue<'a>(site: &'a Content, id_or_key: &str) -> Option<&'a Value> {
    site.data.issues.iter().find(|issue| {
        is_issue(issue, id_or_key)
            && !site
                .hidden
                .contains(issue["id"].as_str().unwrap_or_default())
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
