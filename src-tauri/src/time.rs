//! Timestamps as the app stores them: UTC with milliseconds, so they sort as text.

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::jira::dto::parse_jira_time;

/// A Jira time as stored text; `None` when `value` isn't one.
pub fn utc(value: &Value) -> Option<String> {
    let at = parse_jira_time(value.as_str()?)?;
    Some(format_utc(at.with_timezone(&Utc)))
}

pub fn format_utc(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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
