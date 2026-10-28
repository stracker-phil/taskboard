//! Issues as search and bulk fetch return them. `fields` stays raw JSON: the cache stores it as is and the
//! sync maps what it needs through the accessors.

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// The numeric id as a string; stable when the issue moves to another project, unlike `key`.
    pub id: String,
    pub key: String,
    #[serde(default)]
    pub fields: Value,
    #[serde(default)]
    pub rendered_fields: Value,
}

impl Issue {
    pub fn summary(&self) -> &str {
        self.fields["summary"].as_str().unwrap_or_default()
    }

    pub fn project_id(&self) -> Option<&str> {
        self.fields["project"]["id"].as_str()
    }

    pub fn status_id(&self) -> Option<&str> {
        self.fields["status"]["id"].as_str()
    }

    pub fn updated(&self) -> Option<DateTime<FixedOffset>> {
        self.fields["updated"].as_str().and_then(parse_jira_time)
    }
}

/// Jira's timestamps: `2026-10-20T09:14:03.120+0200`.
pub fn parse_jira_time(text: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f%z").ok()
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    #[serde(default)]
    pub issues: Vec<Issue>,
    #[serde(default)]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub is_last: bool,
}

impl SearchPage {
    /// Jira marks the last page with `isLast`; a page without a token can't be followed either.
    pub fn is_final(&self) -> bool {
        self.is_last || self.next_page_token.is_none()
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BulkFetch {
    #[serde(default)]
    pub issues: Vec<Issue>,
    /// Ids that don't exist or aren't visible to the user.
    #[serde(default)]
    pub issue_errors: Vec<Value>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ApproximateCount {
    pub count: u64,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn issue(value: Value) -> Issue {
        serde_json::from_value(value).unwrap()
    }

    fn page(is_last: bool, token: Option<&str>) -> SearchPage {
        SearchPage {
            issues: Vec::new(),
            next_page_token: token.map(Into::into),
            is_last,
        }
    }

    #[test]
    fn jira_timestamps_parse_with_their_offset() {
        assert_eq!(
            parse_jira_time("2026-10-20T09:14:03.120+0200"),
            Some(DateTime::parse_from_rfc3339("2026-10-20T09:14:03.120+02:00").unwrap())
        );
    }

    #[test]
    fn garbage_is_not_a_jira_timestamp() {
        for text in ["", "yesterday", "2026-10-20", "2026-10-20T09:14:03"] {
            assert_eq!(parse_jira_time(text), None, "{text}");
        }
    }

    #[test]
    fn issue_accessors_read_the_raw_fields() {
        let issue = issue(json!({
            "id": "10042",
            "key": "NW-12",
            "fields": {
                "summary": "Plan the launch",
                "project": { "id": "10001" },
                "status": { "id": "3" },
                "updated": "2026-10-20T09:14:03.120+0200"
            }
        }));
        assert_eq!(issue.id, "10042");
        assert_eq!(issue.key, "NW-12");
        assert_eq!(issue.summary(), "Plan the launch");
        assert_eq!(issue.project_id(), Some("10001"));
        assert_eq!(issue.status_id(), Some("3"));
        assert_eq!(
            issue.updated(),
            parse_jira_time("2026-10-20T09:14:03.120+0200")
        );
    }

    #[test]
    fn an_issue_without_fields_has_empty_accessors() {
        let issue = issue(json!({ "id": "10042", "key": "NW-12" }));
        assert_eq!(issue.summary(), "");
        assert_eq!(issue.project_id(), None);
        assert_eq!(issue.status_id(), None);
        assert_eq!(issue.updated(), None);
    }

    #[test]
    fn a_page_is_final_when_marked_last_or_without_a_token() {
        for (is_last, token, expected) in [
            (true, Some("abc"), true),
            (true, None, true),
            (false, None, true),
            (false, Some("abc"), false),
        ] {
            assert_eq!(
                page(is_last, token).is_final(),
                expected,
                "isLast {is_last}, token {token:?}"
            );
        }
    }

    #[test]
    fn a_search_page_reads_jiras_camel_case_fields() {
        let page: SearchPage = serde_json::from_value(json!({
            "issues": [{ "id": "1", "key": "NW-1" }],
            "nextPageToken": "abc",
            "isLast": false
        }))
        .unwrap();
        assert_eq!(page.issues.len(), 1);
        assert_eq!(page.next_page_token.as_deref(), Some("abc"));
        assert!(!page.is_final());
    }
}
