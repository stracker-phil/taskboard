//! The JQL Taskboard writes: the sync scope and what the sync jobs add to it.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use crate::error::{AppError, AppResult};

/// A JQL string literal.
pub fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `project in ("NW", "LUM") AND (statusCategory != Done OR updated >= -90d)`: open work plus the
/// last `done_days` days of finished work.
pub fn default_scope(project_keys: &[String], done_days: u32) -> AppResult<String> {
    if project_keys.is_empty() {
        return Err(AppError::InvalidInput("Track at least one Space".into()));
    }
    let keys: Vec<String> = project_keys.iter().map(|key| quote(key)).collect();
    Ok(format!(
        "project in ({}) AND (statusCategory != Done OR updated >= -{done_days}d)",
        keys.join(", ")
    ))
}

/// The scope narrowed to one project.
pub fn and_project(scope: &str, project_key: &str) -> String {
    format!("({scope}) AND project = {}", quote(project_key))
}

/// The scope narrowed to issues updated at or after `literal` (see `date_literal`).
pub fn and_updated_since(scope: &str, literal: &str) -> String {
    format!("({scope}) AND updated >= {}", quote(literal))
}

/// Oldest change first, so a sync that stops halfway has a clean high-water mark.
pub fn order_by_updated(jql: &str) -> String {
    format!("{jql} ORDER BY updated ASC")
}

/// `yyyy-MM-dd HH:mm` in the Jira user's time zone (from `/myself`), rounded down to the minute: Jira
/// reads JQL date literals in that zone, not in UTC or the Mac's zone.
pub fn date_literal(instant: DateTime<Utc>, time_zone: &str) -> AppResult<String> {
    let zone: Tz = time_zone
        .parse()
        .map_err(|_| AppError::InvalidInput(format!("Unknown time zone {time_zone}")))?;
    Ok(instant
        .with_timezone(&zone)
        .format("%Y-%m-%d %H:%M")
        .to_string())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap()
    }

    #[test]
    fn the_default_scope_is_open_work_plus_recently_finished_work() {
        assert_eq!(
            default_scope(&["NW".into(), "LUM".into()], 90).unwrap(),
            r#"project in ("NW", "LUM") AND (statusCategory != Done OR updated >= -90d)"#
        );
    }

    #[test]
    fn a_scope_without_projects_is_invalid_input() {
        assert!(matches!(
            default_scope(&[], 90),
            Err(AppError::InvalidInput(_))
        ));
    }

    #[test]
    fn quote_escapes_quotes_and_backslashes() {
        for (input, expected) in [
            ("NW", r#""NW""#),
            (r#"a"b"#, r#""a\"b""#),
            (r"a\b", r#""a\\b""#),
            (r#"\""#, r#""\\\"""#),
        ] {
            assert_eq!(quote(input), expected, "{input}");
        }
    }

    #[test]
    fn clauses_are_added_to_the_scope() {
        let scope = "project = X";
        assert_eq!(
            and_project(scope, "NW"),
            r#"(project = X) AND project = "NW""#
        );
        assert_eq!(
            and_updated_since(scope, "2026-10-28 10:05"),
            r#"(project = X) AND updated >= "2026-10-28 10:05""#
        );
        assert_eq!(
            order_by_updated("project = X"),
            "project = X ORDER BY updated ASC"
        );
    }

    #[test]
    fn date_literals_use_the_jira_users_time_zone() {
        for (instant, zone, expected) in [
            (
                utc(2026, 1, 15, 9, 5, 0),
                "Europe/Berlin",
                "2026-01-15 10:05",
            ),
            (
                utc(2026, 7, 15, 8, 5, 0),
                "Europe/Berlin",
                "2026-07-15 10:05",
            ),
            (
                utc(2026, 1, 15, 18, 5, 0),
                "America/Los_Angeles",
                "2026-01-15 10:05",
            ),
            (
                utc(2026, 10, 28, 4, 35, 0),
                "Asia/Kolkata",
                "2026-10-28 10:05",
            ),
            (utc(2026, 10, 28, 10, 5, 0), "UTC", "2026-10-28 10:05"),
        ] {
            assert_eq!(date_literal(instant, zone).unwrap(), expected, "{zone}");
        }
    }

    #[test]
    fn date_literals_round_down_to_the_minute() {
        assert_eq!(
            date_literal(utc(2026, 10, 28, 9, 5, 59), "Europe/Berlin").unwrap(),
            "2026-10-28 10:05"
        );
    }

    #[test]
    fn date_literals_cross_the_day_boundary_with_the_zone_offset() {
        assert_eq!(
            date_literal(utc(2026, 10, 28, 23, 30, 0), "Europe/Berlin").unwrap(),
            "2026-10-29 00:30"
        );
        assert_eq!(
            date_literal(utc(2026, 10, 28, 3, 30, 0), "America/Los_Angeles").unwrap(),
            "2026-10-27 20:30"
        );
    }

    #[test]
    fn an_unknown_time_zone_is_invalid_input() {
        assert!(matches!(
            date_literal(utc(2026, 10, 28, 10, 5, 0), "Mars/Olympus"),
            Err(AppError::InvalidInput(_))
        ));
    }
}
