//! What of an issue's JSON the cache keeps beside its columns. Pure.
//!
//! `issue.custom` holds the custom fields' values, so views and filters read a small object. `issue_body.raw`
//! drops what other columns and tables keep; comments shrink to their ids and edit times, so an edit still
//! changes it. Rendered fields stay only for custom text fields (`issues::detail` falls back to them): Jira
//! renders dates relative to now, which would make an unchanged issue look changed.

use serde_json::{Map, Value};

use crate::jira::dto::Issue;

fn is_custom(id: &str) -> bool {
    id.starts_with("customfield_")
}

/// The entries of a JSON object that `keep` accepts; empty for anything but an object.
fn pick(object: &Value, keep: impl Fn(&str, &Value) -> bool) -> Map<String, Value> {
    object
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(id, value)| keep(id, value))
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect()
}

/// The custom fields Jira filled, as a JSON object; none when it filled none.
pub fn custom_values(fields: &Value) -> Option<String> {
    let filled = pick(fields, |id, value| is_custom(id) && !value.is_null());
    (!filled.is_empty()).then(|| Value::Object(filled).to_string())
}

/// The issue as `issue_body.raw` keeps it.
pub fn kept_raw(issue: &Issue) -> String {
    let mut fields = pick(&issue.fields, |id, _| !is_custom(id) && id != "description");
    if let Some(comment) = fields.remove("comment").filter(Value::is_object) {
        fields.insert("comment".into(), comment_versions(&comment));
    }
    let rendered = pick(&issue.rendered_fields, |id, value| {
        is_custom(id) && value.is_string()
    });
    serde_json::json!({
        "id": issue.id,
        "key": issue.key,
        "fields": fields,
        "renderedFields": rendered,
    })
    .to_string()
}

fn comment_versions(comment: &Value) -> Value {
    let versions: Vec<Value> = comment["comments"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| serde_json::json!({ "id": c["id"], "updated": c["updated"] }))
        .collect();
    serde_json::json!({ "total": comment["total"], "comments": versions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn issue(fields: Value, rendered: Value) -> Issue {
        serde_json::from_value(json!({
            "id": "101042",
            "key": "NW-1042",
            "fields": fields,
            "renderedFields": rendered,
        }))
        .unwrap()
    }

    fn kept(issue: &Issue) -> Value {
        serde_json::from_str(&kept_raw(issue)).unwrap()
    }

    fn commented(updated: &str) -> Issue {
        issue(
            json!({
                "summary": "Validate postcodes",
                "comment": {
                    "total": 1,
                    "comments": [{ "id": "30001", "updated": updated, "body": "Looks good" }]
                }
            }),
            json!({}),
        )
    }

    #[test]
    fn only_filled_custom_fields_are_kept_as_a_json_object() {
        let fields = json!({
            "summary": "Validate postcodes",
            "customfield_10001": "North",
            "customfield_10002": null,
            "customfield_10003": { "accountId": "u-1" },
        });

        let custom: Value = serde_json::from_str(&custom_values(&fields).unwrap()).unwrap();

        assert_eq!(
            custom,
            json!({ "customfield_10001": "North", "customfield_10003": { "accountId": "u-1" } })
        );
    }

    #[test]
    fn no_custom_values_are_kept_when_jira_filled_none() {
        for fields in [
            json!({ "summary": "Validate postcodes" }),
            json!({ "customfield_10001": null }),
            json!({}),
            json!(null),
        ] {
            assert_eq!(custom_values(&fields), None, "{fields}");
        }
    }

    #[test]
    fn the_kept_raw_drops_what_other_columns_and_tables_hold() {
        let fetched = issue(
            json!({
                "summary": "Validate postcodes",
                "description": { "type": "doc", "version": 1, "content": [] },
                "attachment": [{ "id": "80001", "filename": "plan.png" }],
                "issuelinks": [{ "id": "90001" }],
                "comment": {
                    "total": 1,
                    "comments": [{ "id": "30001", "updated": "2026-10-27T09:00:00.000+0000", "body": "Long" }]
                },
                "customfield_10001": "North",
            }),
            json!({
                "description": "<p>Hello</p>",
                "updated": "2 hours ago",
                "customfield_10001": "<p>North</p>",
                "customfield_10002": null,
            }),
        );

        assert_eq!(
            kept(&fetched),
            json!({
                "id": "101042",
                "key": "NW-1042",
                "fields": {
                    "summary": "Validate postcodes",
                    "attachment": [{ "id": "80001", "filename": "plan.png" }],
                    "issuelinks": [{ "id": "90001" }],
                    "comment": {
                        "total": 1,
                        "comments": [{ "id": "30001", "updated": "2026-10-27T09:00:00.000+0000" }]
                    }
                },
                "renderedFields": { "customfield_10001": "<p>North</p>" }
            })
        );
    }

    #[test]
    fn an_issue_without_comments_or_rendered_fields_keeps_empty_objects() {
        let bare = issue(json!({ "summary": "Validate postcodes" }), json!(null));

        assert_eq!(
            kept(&bare),
            json!({
                "id": "101042",
                "key": "NW-1042",
                "fields": { "summary": "Validate postcodes" },
                "renderedFields": {}
            })
        );
    }

    #[test]
    fn two_fetches_differing_only_in_a_relative_rendered_date_keep_the_same_raw() {
        let at = |rendered: &str| {
            issue(
                json!({ "summary": "Validate postcodes" }),
                json!({ "updated": rendered, "created": rendered }),
            )
        };

        assert_eq!(kept_raw(&at("2 hours ago")), kept_raw(&at("3 hours ago")));
    }

    #[test]
    fn an_edited_comment_changes_the_kept_raw() {
        assert_ne!(
            kept_raw(&commented("2026-10-27T09:00:00.000+0000")),
            kept_raw(&commented("2026-10-28T09:00:00.000+0000"))
        );
    }
}
