//! `GET /field`: the system fields the app requests plus the demo site's custom fields.

use serde_json::{Value, json};

pub const SPRINT: &str = "customfield_10020";
pub const EPIC_COLOR: &str = "customfield_10017";
pub const STORY_POINTS: &str = "customfield_10016";
pub const REVIEWER: &str = "customfield_10071";
/// Rich text fields of the bug template.
pub const STEPS: &str = "customfield_10080";
pub const ACCEPTANCE: &str = "customfield_10081";

pub fn fields() -> Vec<Value> {
    let mut all: Vec<Value> = SYSTEM
        .iter()
        .map(|(id, name, kind, items)| system(id, name, kind, *items))
        .collect();
    all.extend([
        custom(
            10020,
            "Sprint",
            json!({ "type": "array", "items": "json" }),
            "com.pyxis.greenhopper.jira:gh-sprint",
        ),
        custom(
            10017,
            "Epic Color",
            json!({ "type": "any" }),
            "com.pyxis.greenhopper.jira:gh-epic-color",
        ),
        custom(
            10016,
            "Story points",
            json!({ "type": "number" }),
            "com.atlassian.jira.plugin.system.customfieldtypes:float",
        ),
        custom(
            10071,
            "Reviewer",
            json!({ "type": "user" }),
            "com.atlassian.jira.plugin.system.customfieldtypes:userpicker",
        ),
        custom(
            10080,
            "Steps to reproduce",
            json!({ "type": "string" }),
            "com.atlassian.jira.plugin.system.customfieldtypes:textarea",
        ),
        custom(
            10081,
            "Acceptance criteria",
            json!({ "type": "string" }),
            "com.atlassian.jira.plugin.system.customfieldtypes:textarea",
        ),
        // Company-wide fields no tracked project fills: the field pickers leave them out.
        custom(
            10090,
            "Cost centre",
            json!({ "type": "option" }),
            "com.atlassian.jira.plugin.system.customfieldtypes:select",
        ),
        custom(
            10091,
            "Legal approval date",
            json!({ "type": "date" }),
            "com.atlassian.jira.plugin.system.customfieldtypes:datepicker",
        ),
        custom(
            10019,
            "Rank",
            json!({ "type": "any" }),
            "com.pyxis.greenhopper.jira:gh-lexo-rank",
        ),
    ]);
    all
}

/// (id, name, schema type, array item type)
const SYSTEM: [(&str, &str, &str, Option<&str>); 16] = [
    ("summary", "Summary", "string", None),
    ("status", "Status", "status", None),
    ("issuetype", "Issue Type", "issuetype", None),
    ("priority", "Priority", "priority", None),
    ("assignee", "Assignee", "user", None),
    ("reporter", "Reporter", "user", None),
    ("parent", "Parent", "", None),
    ("labels", "Labels", "array", Some("string")),
    ("duedate", "Due date", "date", None),
    ("fixVersions", "Fix versions", "array", Some("version")),
    ("created", "Created", "datetime", None),
    ("updated", "Updated", "datetime", None),
    ("resolutiondate", "Resolved", "datetime", None),
    ("project", "Project", "project", None),
    ("comment", "Comment", "comments-page", None),
    ("description", "Description", "string", None),
];

fn system(id: &str, name: &str, kind: &str, items: Option<&str>) -> Value {
    let mut value = json!({
        "id": id,
        "key": id,
        "name": name,
        "custom": false,
        "orderable": true,
        "navigable": true,
        "searchable": true,
        "clauseNames": [id],
    });
    // Jira sends no schema for the parent field.
    if !kind.is_empty() {
        let mut schema = json!({ "type": kind, "system": id });
        if let Some(items) = items {
            schema["items"] = json!(items);
        }
        value["schema"] = schema;
    }
    value
}

fn custom(number: u64, name: &str, mut schema: Value, plugin: &str) -> Value {
    let id = format!("customfield_{number}");
    schema["custom"] = json!(plugin);
    schema["customId"] = json!(number);
    json!({
        "id": id,
        "key": id,
        "name": name,
        "untranslatedName": name,
        "custom": true,
        "orderable": true,
        "navigable": true,
        "searchable": true,
        "clauseNames": [format!("cf[{number}]"), name],
        "schema": schema,
    })
}
