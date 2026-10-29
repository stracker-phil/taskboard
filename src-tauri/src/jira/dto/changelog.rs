//! An issue's changelog: who changed which field when, as `GET /issue/{id}/changelog` pages it.

use serde::Deserialize;

use super::meta::User;

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangelogPage {
    #[serde(default)]
    pub values: Vec<Changelog>,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub is_last: bool,
}

/// One edit: every field it changed, at one time, by one person.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Changelog {
    pub id: String,
    /// Missing for automation and anonymous edits.
    #[serde(default)]
    pub author: Option<User>,
    /// Jira's time, with its offset.
    pub created: String,
    #[serde(default)]
    pub items: Vec<ChangeItem>,
}

/// One field's change. `from`/`to` hold ids (status, account, version) where the field has them;
/// `fromString`/`toString` the text Jira shows.
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeItem {
    /// The field's display name (`status`, `Fix Version`, `Sprint`).
    pub field: String,
    #[serde(default)]
    pub field_id: Option<String>,
    #[serde(default)]
    pub fieldtype: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default, rename = "fromString")]
    pub from_text: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default, rename = "toString")]
    pub to_text: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_changelog_page_keeps_ids_and_texts_and_leaves_missing_parts_empty() {
        let page: ChangelogPage = serde_json::from_value(json!({
            "startAt": 0,
            "maxResults": 100,
            "total": 2,
            "isLast": true,
            "values": [
                {
                    "id": "80001",
                    "author": {
                        "accountId": "712020:aaaa",
                        "displayName": "Alex Morgan",
                        "active": true
                    },
                    "created": "2026-10-23T09:30:00.000+0200",
                    "items": [{
                        "field": "status",
                        "fieldtype": "jira",
                        "fieldId": "status",
                        "from": "10000",
                        "fromString": "To Do",
                        "to": "10001",
                        "toString": "In Progress"
                    }]
                },
                {
                    "id": "80002",
                    "created": "2026-10-24T10:00:00.000+0200",
                    "items": [{
                        "field": "assignee",
                        "from": null,
                        "fromString": null,
                        "to": "712020:aaaa",
                        "toString": "Alex Morgan"
                    }]
                }
            ]
        }))
        .unwrap();

        assert_eq!(page.total, 2);
        assert!(page.is_last);
        assert_eq!(
            page.values,
            [
                Changelog {
                    id: "80001".into(),
                    author: Some(User {
                        account_id: "712020:aaaa".into(),
                        display_name: "Alex Morgan".into(),
                        active: true,
                        avatar_urls: Default::default(),
                    }),
                    created: "2026-10-23T09:30:00.000+0200".into(),
                    items: vec![ChangeItem {
                        field: "status".into(),
                        field_id: Some("status".into()),
                        fieldtype: Some("jira".into()),
                        from: Some("10000".into()),
                        from_text: Some("To Do".into()),
                        to: Some("10001".into()),
                        to_text: Some("In Progress".into()),
                    }],
                },
                Changelog {
                    id: "80002".into(),
                    author: None,
                    created: "2026-10-24T10:00:00.000+0200".into(),
                    items: vec![ChangeItem {
                        field: "assignee".into(),
                        field_id: None,
                        fieldtype: None,
                        from: None,
                        from_text: None,
                        to: Some("712020:aaaa".into()),
                        to_text: Some("Alex Morgan".into()),
                    }],
                },
            ]
        );
    }

    #[test]
    fn an_empty_changelog_page_has_no_entries_and_is_not_last() {
        let page: ChangelogPage = serde_json::from_value(json!({})).unwrap();

        assert_eq!(page.values, []);
        assert_eq!(page.total, 0);
        assert!(!page.is_last);
    }
}
