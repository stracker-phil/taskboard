//! Jira's issue JSON → the cache's rows. Pure: no database, no clock.

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::jira::dto::{Issue, parse_jira_time};
use crate::net::AvatarUrl;

/// The custom fields whose ids differ per site (found in `/field` by their type).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldIds {
    pub sprint: Option<String>,
    pub epic_color: Option<String>,
}

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
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommentRow {
    pub id: String,
    pub issue_id: String,
    pub author_id: Option<String>,
    pub body_html: Option<String>,
    pub body_adf: Option<String>,
    pub body_text: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserRow {
    pub account_id: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

/// The text the search index keeps for one issue.
#[derive(Debug, Clone, PartialEq)]
pub struct FtsDoc {
    pub rowid: i64,
    pub key: String,
    pub summary: String,
    pub description: String,
    pub comments: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mapped {
    pub issue: IssueRow,
    pub comments: Vec<CommentRow>,
    pub users: Vec<UserRow>,
    pub fts: FtsDoc,
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
        raw: serde_json::to_string(issue).unwrap_or_default(),
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

fn text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// UTC with milliseconds, so stored timestamps sort as text.
pub fn utc(value: &Value) -> Option<String> {
    let at = parse_jira_time(value.as_str()?)?;
    Some(format_utc(at.with_timezone(&Utc)))
}

pub fn format_utc(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
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

fn comments_of(issue: &Issue) -> Vec<CommentRow> {
    let rendered = issue.rendered_fields["comment"]["comments"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let html_of = |id: &str| {
        rendered
            .iter()
            .find(|c| c["id"] == id)
            .and_then(|c| text(&c["body"]))
    };
    issue.fields["comment"]["comments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let id = text(&c["id"])?;
            let body_html = html_of(&id);
            Some(CommentRow {
                body_text: body_html.as_deref().map(html_to_text).unwrap_or_default(),
                body_html,
                body_adf: Some(&c["body"])
                    .filter(|b| b.is_object())
                    .map(Value::to_string),
                issue_id: issue.id.clone(),
                author_id: text(&c["author"]["accountId"]),
                created_at: utc(&c["created"])?,
                updated_at: utc(&c["updated"])?,
                id,
            })
        })
        .collect()
}

fn users_of(issue: &Issue) -> Vec<UserRow> {
    let f = &issue.fields;
    let mut people: Vec<&Value> = vec![&f["assignee"], &f["reporter"]];
    people.extend(
        f["comment"]["comments"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| &c["author"]),
    );
    let mut users: Vec<UserRow> = Vec::new();
    for person in people {
        let (Some(account_id), Some(name)) =
            (text(&person["accountId"]), text(&person["displayName"]))
        else {
            continue;
        };
        if users.iter().all(|u| u.account_id != account_id) {
            users.push(UserRow {
                account_id,
                name,
                avatar_url: text(&person["avatarUrls"]["48x48"])
                    .filter(|url| AvatarUrl::parse(url).is_some()),
            });
        }
    }
    users
}

/// Jira's rendered HTML as plain text for search: every tag becomes a space, common entities are decoded.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::test_support::demo::{EPIC_COLOR, SPRINT, demo};
    use serde_json::json;

    const ALEX: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e01";
    const JONAS: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e02";
    const MARTA: &str = "712020:5a1e0c3f-8d2b-4f61-9c7e-1b2a3c4d5e03";

    const POSTCODES_HTML: &str = "<p>Validate the postcode against the selected country before the \
        payment step. Austria and Switzerland use 4 digits, Germany uses 5.</p><ul>\
        <li>Show inline errors under the field, not in a toast.</li>\
        <li>Unknown countries pass without validation.</li>\
        <li>Guest checkout must never be blocked by this.</li></ul>\
        <p><img alt=\"screenshot-login.png\"></p>";

    const POSTCODES_TEXT: &str = "Validate the postcode against the selected country before the \
        payment step. Austria and Switzerland use 4 digits, Germany uses 5. Show inline errors \
        under the field, not in a toast. Unknown countries pass without validation. Guest \
        checkout must never be blocked by this.";

    fn ids() -> FieldIds {
        FieldIds {
            sprint: Some(SPRINT.into()),
            epic_color: Some(EPIC_COLOR.into()),
        }
    }

    fn issue_json(key: &str) -> Value {
        demo()
            .issues
            .into_iter()
            .find(|issue| issue["key"] == key)
            .unwrap_or_else(|| panic!("no demo issue {key}"))
    }

    fn demo_issue(key: &str) -> Issue {
        serde_json::from_value(issue_json(key)).unwrap()
    }

    fn edited(key: &str, edit: impl FnOnce(&mut Value)) -> Issue {
        let mut issue = issue_json(key);
        edit(&mut issue);
        serde_json::from_value(issue).unwrap()
    }

    fn mapped(key: &str) -> Mapped {
        map_issue(&demo_issue(key), &ids()).unwrap_or_else(|| panic!("{key} didn't map"))
    }

    fn mapped_edit(key: &str, edit: impl FnOnce(&mut Value)) -> Mapped {
        map_issue(&edited(key, edit), &ids()).unwrap_or_else(|| panic!("{key} didn't map"))
    }

    /// The demo people's pictures are on `avatar.example.com`, which isn't Atlassian's avatar host.
    fn user_row(account_id: &str, name: &str) -> UserRow {
        UserRow {
            account_id: account_id.into(),
            name: name.into(),
            avatar_url: None,
        }
    }

    fn assignee_avatar(url: &str) -> Option<String> {
        let mapped = mapped_edit("NW-1042", |issue| {
            issue["fields"]["assignee"]["avatarUrls"]["48x48"] = json!(url);
        });
        mapped
            .users
            .into_iter()
            .find(|u| u.account_id == ALEX)
            .and_then(|u| u.avatar_url)
    }

    type SprintColumns = (Option<String>, Option<String>, Option<String>);

    fn chosen_sprint(issue: &Issue, ids: &FieldIds) -> SprintColumns {
        let row = map_issue(issue, ids).unwrap().issue;
        (row.sprint, row.sprint_id, row.sprint_state)
    }

    fn sprint(name: &str, id: &str, state: &str) -> SprintColumns {
        (Some(name.into()), Some(id.into()), Some(state.into()))
    }

    #[test]
    fn an_issue_maps_to_every_column_of_its_row() {
        let row = mapped("NW-1042").issue;

        assert_eq!(
            IssueRow {
                raw: String::new(),
                description_adf: None,
                ..row
            },
            IssueRow {
                id: "101042".into(),
                key: "NW-1042".into(),
                project_id: "10001".into(),
                parent_id: Some("101020".into()),
                fix_versions: r#"["10101"]"#.into(),
                epic_color: None,
                summary: "Checkout: address validation for DACH postcodes".into(),
                issue_type: "Story".into(),
                type_is_subtask: false,
                is_epic: false,
                status_id: "3".into(),
                status_category: "indeterminate".into(),
                priority: Some("High".into()),
                assignee_id: Some(ALEX.into()),
                reporter_id: Some(JONAS.into()),
                sprint: Some("Sprint 22".into()),
                sprint_id: Some("22".into()),
                sprint_state: Some("active".into()),
                labels: r#"["frontend"]"#.into(),
                due_date: Some("2026-10-30".into()),
                created_at: "2026-10-12T07:14:00.000Z".into(),
                updated_at: "2026-10-28T10:42:00.000Z".into(),
                resolved_at: None,
                description_html: Some(POSTCODES_HTML.into()),
                description_adf: None,
                comment_count: 4,
                comments_complete: true,
                raw: String::new(),
            }
        );
    }

    #[test]
    fn the_raw_column_is_the_issue_json_and_the_adf_column_is_its_description_document() {
        let issue = demo_issue("NW-1042");

        let row = map_issue(&issue, &ids()).unwrap().issue;

        assert_eq!(row.raw, serde_json::to_string(&issue).unwrap());
        let adf: Value = serde_json::from_str(row.description_adf.as_deref().unwrap()).unwrap();
        assert_eq!(adf, issue.fields["description"]);
        assert_eq!(adf["type"], "doc");
        assert_eq!(adf["content"][0]["type"], "paragraph");
        assert_eq!(adf["content"][1]["type"], "bulletList");
        assert_eq!(
            adf["content"][1]["content"].as_array().map(Vec::len),
            Some(3)
        );
    }

    #[test]
    fn times_are_utc_with_milliseconds_across_the_daylight_saving_change() {
        let done = mapped("NW-1043").issue;

        assert_eq!(done.created_at, "2026-10-23T08:00:00.000Z");
        assert_eq!(done.updated_at, "2026-10-26T16:45:00.000Z");
        assert_eq!(
            done.resolved_at.as_deref(),
            Some("2026-10-26T16:45:00.000Z")
        );
        assert_eq!(done.status_category, "done");
    }

    #[test]
    fn an_issue_without_optional_fields_maps_empty_columns() {
        let row = mapped_edit("NW-1042", |issue| {
            for name in [
                "assignee",
                "priority",
                "duedate",
                "labels",
                "fixVersions",
                "description",
                "resolutiondate",
                "parent",
                SPRINT,
            ] {
                issue["fields"][name] = Value::Null;
            }
            issue["renderedFields"]["description"] = Value::Null;
        })
        .issue;

        assert_eq!(row.assignee_id, None);
        assert_eq!(row.priority, None);
        assert_eq!(row.due_date, None);
        assert_eq!(row.labels, "[]");
        assert_eq!(row.fix_versions, "[]");
        assert_eq!(row.description_html, None);
        assert_eq!(row.description_adf, None);
        assert_eq!(row.resolved_at, None);
        assert_eq!(row.parent_id, None);
        assert_eq!(row.sprint_id, None);
    }

    #[test]
    fn an_empty_rendered_description_is_no_description() {
        let row = mapped_edit("NW-1042", |issue| {
            issue["renderedFields"]["description"] = json!("");
        })
        .issue;

        assert_eq!(row.description_html, None);
    }

    #[test]
    fn a_site_without_the_custom_fields_maps_no_sprint_and_no_epic_color() {
        let none = FieldIds::default();

        assert_eq!(
            chosen_sprint(&demo_issue("NW-1042"), &none),
            (None, None, None)
        );
        let epic = map_issue(&demo_issue("NW-1020"), &none).unwrap().issue;
        assert!(epic.is_epic);
        assert_eq!(epic.epic_color, None);
    }

    #[test]
    fn the_active_sprint_wins_over_older_and_newer_ones() {
        for key in ["NW-1042", "NW-1033", "NW-1038"] {
            assert_eq!(
                chosen_sprint(&demo_issue(key), &ids()),
                sprint("Sprint 22", "22", "active"),
                "{key}"
            );
        }
        let reversed = edited("NW-1033", |issue| {
            issue["fields"][SPRINT].as_array_mut().unwrap().reverse();
        });
        assert_eq!(
            chosen_sprint(&reversed, &ids()),
            sprint("Sprint 22", "22", "active")
        );
    }

    #[test]
    fn without_an_active_sprint_the_highest_id_wins_in_any_order() {
        let closed_21 = json!({ "id": 21, "name": "Sprint 21", "state": "closed" });
        let closed_37 = json!({ "id": 37, "name": "LUM Sprint 8", "state": "closed" });
        let future_23 = json!({ "id": 23, "name": "Sprint 23", "state": "future" });
        for (sprints, expected) in [
            (
                vec![closed_21.clone(), closed_37.clone()],
                sprint("LUM Sprint 8", "37", "closed"),
            ),
            (
                vec![closed_37.clone(), closed_21.clone()],
                sprint("LUM Sprint 8", "37", "closed"),
            ),
            (
                vec![closed_21.clone(), future_23.clone()],
                sprint("Sprint 23", "23", "future"),
            ),
            (vec![closed_21.clone()], sprint("Sprint 21", "21", "closed")),
        ] {
            let issue = edited("NW-1042", |issue| issue["fields"][SPRINT] = json!(sprints));
            assert_eq!(chosen_sprint(&issue, &ids()), expected, "{sprints:?}");
        }
    }

    #[test]
    fn demo_issues_in_one_closed_or_one_future_sprint_pick_it() {
        assert_eq!(
            chosen_sprint(&demo_issue("NW-1024"), &ids()),
            sprint("Sprint 21", "21", "closed")
        );
        assert_eq!(
            chosen_sprint(&demo_issue("NW-1051"), &ids()),
            sprint("Sprint 23", "23", "future")
        );
    }

    #[test]
    fn an_issue_without_sprints_has_no_sprint_columns() {
        assert_eq!(
            chosen_sprint(&demo_issue("NW-1055"), &ids()),
            (None, None, None)
        );
        let empty = edited("NW-1042", |issue| issue["fields"][SPRINT] = json!([]));
        assert_eq!(chosen_sprint(&empty, &ids()), (None, None, None));
    }

    #[test]
    fn an_epic_keeps_its_color() {
        let row = mapped("NW-1020").issue;

        assert!(row.is_epic);
        assert_eq!(row.issue_type, "Epic");
        assert_eq!(row.epic_color.as_deref(), Some("ghx-label-7"));
        assert_eq!(row.parent_id, None);
        assert!(!row.type_is_subtask);
    }

    #[test]
    fn a_story_has_no_epic_color_even_when_the_field_is_set() {
        let row = mapped_edit("NW-1042", |issue| {
            issue["fields"][EPIC_COLOR] = json!("ghx-label-3");
        })
        .issue;

        assert!(!row.is_epic);
        assert_eq!(row.epic_color, None);
    }

    #[test]
    fn an_issue_type_is_an_epic_by_its_level_or_by_its_name() {
        for (issue_type, is_epic) in [
            (json!({ "name": "Epic", "hierarchyLevel": 1 }), true),
            (json!({ "name": "Epic" }), true),
            (json!({ "name": "Initiative", "hierarchyLevel": 1 }), true),
            (json!({ "name": "Story", "hierarchyLevel": 0 }), false),
            (
                json!({ "name": "Subtask", "hierarchyLevel": -1, "subtask": true }),
                false,
            ),
        ] {
            let row = mapped_edit("NW-1042", |issue| {
                issue["fields"]["issuetype"] = issue_type.clone();
            })
            .issue;
            assert_eq!(row.is_epic, is_epic, "{issue_type}");
        }
    }

    #[test]
    fn a_subtask_knows_it_is_one_and_its_parent() {
        let row = mapped("NW-1059").issue;

        assert!(row.type_is_subtask);
        assert!(!row.is_epic);
        assert_eq!(row.issue_type, "Subtask");
        assert_eq!(row.parent_id.as_deref(), Some("101042"));
        assert_eq!(row.key, "NW-1059");
    }

    #[test]
    fn a_story_under_an_epic_has_the_epic_as_parent_and_is_no_subtask() {
        let row = mapped("NW-1042").issue;

        assert_eq!(row.parent_id.as_deref(), Some("101020"));
        assert!(!row.type_is_subtask);
    }

    #[test]
    fn comments_become_rows_with_their_authors_html_and_plain_text() {
        let comments = mapped("NW-1042").comments;

        let rows: Vec<(Option<&str>, Option<&str>, &str, &str)> = comments
            .iter()
            .map(|c| {
                (
                    c.author_id.as_deref(),
                    c.body_html.as_deref(),
                    c.body_text.as_str(),
                    c.created_at.as_str(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                (
                    Some(MARTA),
                    Some("<p>Design for the error states is ready for review.</p>"),
                    "Design for the error states is ready for review.",
                    "2026-10-13T08:30:00.000Z"
                ),
                (
                    Some(ALEX),
                    Some("<p>Validation done for AT, DE and CH. Tests are green on staging.</p>"),
                    "Validation done for AT, DE and CH. Tests are green on staging.",
                    "2026-10-26T16:48:00.000Z"
                ),
                (
                    Some(MARTA),
                    Some(
                        "<p>Copy for the error states is in the design file, page \
                         \u{201c}Checkout\u{201d}.</p>"
                    ),
                    "Copy for the error states is in the design file, page \
                     \u{201c}Checkout\u{201d}.",
                    "2026-10-28T09:05:00.000Z"
                ),
                (
                    Some(MARTA),
                    Some(
                        "<p>Client confirmed Liechtenstein should behave like Switzerland. \
                         Can you include it in the rules?</p>"
                    ),
                    "Client confirmed Liechtenstein should behave like Switzerland. \
                     Can you include it in the rules?",
                    "2026-10-28T10:42:00.000Z"
                ),
            ]
        );
        assert!(comments.iter().all(|c| c.issue_id == "101042"));
        assert!(comments.iter().all(|c| c.updated_at == c.created_at));
        let distinct: HashSet<&str> = comments.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(distinct.len(), 4);
    }

    #[test]
    fn a_comment_row_carries_its_body_adf_as_json() {
        let issue = demo_issue("NW-1047");

        let comments = map_issue(&issue, &ids()).unwrap().comments;

        assert_eq!(comments.len(), 1);
        let adf: Value = serde_json::from_str(comments[0].body_adf.as_deref().unwrap()).unwrap();
        assert_eq!(adf, issue.fields["comment"]["comments"][0]["body"]);
        assert_eq!(adf["type"], "doc");
    }

    #[test]
    fn a_comment_whose_body_is_not_an_object_has_no_adf() {
        for body in [json!("Plain text"), json!(["a"]), Value::Null] {
            let mapped = mapped_edit("NW-1042", |issue| {
                for comment in issue["fields"]["comment"]["comments"]
                    .as_array_mut()
                    .unwrap()
                {
                    comment["body"] = body.clone();
                }
            });

            assert_eq!(mapped.comments.len(), 4, "{body}");
            assert!(
                mapped.comments.iter().all(|c| c.body_adf.is_none()),
                "{body}"
            );
            assert!(
                mapped.comments.iter().all(|c| c.body_html.is_some()),
                "{body}"
            );
        }
    }

    #[test]
    fn a_comment_without_rendered_html_keeps_empty_text() {
        let mapped = mapped_edit("NW-1042", |issue| {
            issue["renderedFields"]["comment"]["comments"] = json!([]);
        });

        assert_eq!(mapped.comments.len(), 4);
        assert!(mapped.comments.iter().all(|c| c.body_html.is_none()));
        assert!(mapped.comments.iter().all(|c| c.body_text.is_empty()));
    }

    #[test]
    fn an_issue_without_comments_has_none() {
        let mapped = mapped("NW-1003");

        assert_eq!(mapped.comments, vec![]);
        assert_eq!(mapped.issue.comment_count, 0);
        assert!(mapped.issue.comments_complete);
        assert_eq!(mapped.fts.comments, "");
    }

    #[test]
    fn comments_are_complete_when_the_total_equals_the_listed_ones() {
        let row = mapped("NW-1042").issue;

        assert_eq!(row.comment_count, 4);
        assert!(row.comments_complete);
    }

    #[test]
    fn comments_are_incomplete_when_jira_counts_more_than_it_listed() {
        let mapped = mapped_edit("NW-1042", |issue| {
            issue["fields"]["comment"]["total"] = json!(9);
        });

        assert_eq!(mapped.issue.comment_count, 9);
        assert!(!mapped.issue.comments_complete);
        assert_eq!(mapped.comments.len(), 4);
    }

    #[test]
    fn an_issue_without_a_comment_field_counts_none_and_is_complete() {
        let mapped = mapped_edit("NW-1042", |issue| {
            issue["fields"]["comment"] = Value::Null;
            issue["renderedFields"]["comment"] = Value::Null;
        });

        assert_eq!(mapped.issue.comment_count, 0);
        assert!(mapped.issue.comments_complete);
        assert_eq!(mapped.comments, vec![]);
    }

    #[test]
    fn people_are_listed_once_in_the_order_assignee_reporter_commenters() {
        let users = mapped("NW-1042").users;

        assert_eq!(
            users,
            vec![
                user_row(ALEX, "Alex Morgan"),
                user_row(JONAS, "Jonas Reiter"),
                user_row(MARTA, "Marta Klein"),
            ]
        );
    }

    #[test]
    fn a_person_keeps_their_48_px_picture_on_the_atlassian_avatar_host() {
        let url = "https://avatar-management--avatars.us-west-2.prod.public.atl-paas.net/712020:aaaa/0f1e2d3c/48";

        assert_eq!(assignee_avatar(url).as_deref(), Some(url));
    }

    #[test]
    fn a_picture_on_any_other_host_is_not_kept() {
        for url in [
            "https://secure.gravatar.com/avatar/0f1e2d3c?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FAM-1.png",
            "http://avatar-management--avatars.us-west-2.prod.public.atl-paas.net/712020:aaaa/0f1e2d3c/48",
            "https://avatar-management--avatars.evil.example.com/712020:aaaa/48",
            "https://example.atlassian.net/secure/useravatar?avatarId=10122",
            "not a url",
        ] {
            assert_eq!(assignee_avatar(url), None, "{url}");
        }
    }

    #[test]
    fn a_person_without_a_display_name_is_not_a_user_row_but_stays_the_assignee() {
        let mapped = mapped_edit("NW-1042", |issue| {
            issue["fields"]["assignee"] = json!({ "accountId": "deleted-user-1" });
        });

        assert_eq!(mapped.issue.assignee_id.as_deref(), Some("deleted-user-1"));
        let ids: Vec<&str> = mapped.users.iter().map(|u| u.account_id.as_str()).collect();
        assert_eq!(ids, [JONAS, MARTA, ALEX]);
    }

    #[test]
    fn the_search_document_has_the_key_summary_and_plain_texts() {
        let fts = mapped("NW-1042").fts;

        assert_eq!(
            fts,
            FtsDoc {
                rowid: 101042,
                key: "NW-1042".into(),
                summary: "Checkout: address validation for DACH postcodes".into(),
                description: POSTCODES_TEXT.into(),
                comments: "Design for the error states is ready for review.\n\
                    Validation done for AT, DE and CH. Tests are green on staging.\n\
                    Copy for the error states is in the design file, page \
                    \u{201c}Checkout\u{201d}.\n\
                    Client confirmed Liechtenstein should behave like Switzerland. \
                    Can you include it in the rules?"
                    .into(),
            }
        );
    }

    #[test]
    fn an_issue_without_description_has_an_empty_search_description() {
        assert_eq!(mapped("NW-1003").fts.description, "");
    }

    #[test]
    fn an_issue_lacking_what_every_row_needs_is_not_mapped() {
        type Edit = fn(&mut Value);
        let cases: [(&str, Edit); 7] = [
            ("no project", |v| v["fields"]["project"] = Value::Null),
            ("no status", |v| v["fields"]["status"] = Value::Null),
            ("status without category", |v| {
                v["fields"]["status"]["statusCategory"] = Value::Null
            }),
            ("no created", |v| v["fields"]["created"] = Value::Null),
            ("no updated", |v| v["fields"]["updated"] = Value::Null),
            ("unreadable updated", |v| {
                v["fields"]["updated"] = json!("yesterday")
            }),
            ("non-numeric id", |v| v["id"] = json!("abc")),
        ];
        for (label, edit) in cases {
            assert_eq!(map_issue(&edited("NW-1042", edit), &ids()), None, "{label}");
        }
    }

    #[test]
    fn every_demo_issue_maps() {
        let issues = demo().issues;
        assert!(!issues.is_empty());
        for value in issues {
            let issue: Issue = serde_json::from_value(value).unwrap();
            assert!(map_issue(&issue, &ids()).is_some(), "{}", issue.key);
        }
    }

    #[test]
    fn html_becomes_plain_text() {
        for (html, text) in [
            ("", ""),
            ("plain", "plain"),
            ("<p>Hello <b>world</b></p>", "Hello world"),
            ("<ul><li>One</li><li>Two</li></ul>", "One Two"),
            ("<p>a</p>\n\n<p>  b \t c </p>", "a b c"),
            ("Tom &amp; Jerry", "Tom & Jerry"),
            (
                "&lt;b&gt; &quot;x&quot; it&#39;s&nbsp;fine",
                "<b> \"x\" it's fine",
            ),
            ("&amp;lt;", "&lt;"),
        ] {
            assert_eq!(html_to_text(html), text, "{html:?}");
        }
    }

    #[test]
    fn jira_times_become_utc_text_with_milliseconds() {
        for (jira, utc_text) in [
            ("2026-10-28T11:42:00.000+0100", "2026-10-28T10:42:00.000Z"),
            ("2026-10-12T09:14:03.120+0200", "2026-10-12T07:14:03.120Z"),
            ("2026-10-28T02:30:00.500-0500", "2026-10-28T07:30:00.500Z"),
            ("2026-10-28T10:42:00.000+0000", "2026-10-28T10:42:00.000Z"),
        ] {
            assert_eq!(utc(&json!(jira)).as_deref(), Some(utc_text), "{jira}");
        }
    }

    #[test]
    fn something_that_is_not_a_jira_time_is_none() {
        for value in [json!(null), json!(42), json!("yesterday"), json!("")] {
            assert_eq!(utc(&value), None, "{value}");
        }
    }
}
