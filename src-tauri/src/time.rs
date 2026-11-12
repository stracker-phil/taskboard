//! Timestamps as the app stores them: UTC with milliseconds, so they sort as text; and the owner's local days.

use chrono::{DateTime, Days, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use crate::jira::dto::parse_jira_time;

/// A Jira time as stored text; `None` when `value` isn't one.
pub fn utc(value: &Value) -> Option<String> {
    utc_text(value.as_str()?)
}

/// `utc` for a time that is already text.
pub fn utc_text(text: &str) -> Option<String> {
    let at = parse_jira_time(text)?;
    Some(format_utc(at.with_timezone(&Utc)))
}

pub fn format_utc(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// The longest stretch of local time a clock change skips, with room to spare.
const MAX_SKIPPED_HOURS: i64 = 3;

/// Local midnight; on a day whose midnight a clock change skips, the first local time that exists.
pub fn day_start(date: NaiveDate, tz: Tz) -> DateTime<Utc> {
    let midnight = date.and_time(NaiveTime::MIN);
    (0..=MAX_SKIPPED_HOURS)
        .find_map(|hours| {
            tz.from_local_datetime(&(midnight + Duration::hours(hours)))
                .earliest()
        })
        .map_or_else(|| midnight.and_utc(), |at| at.with_timezone(&Utc))
}

/// Where the owner's yesterday, today and tomorrow start (UTC text), and their zone, so the page can say "today"
/// or "yesterday" without its own day math (ADR 0074). Read again once `tomorrow` has begun.
#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct LocalDays {
    pub yesterday: String,
    pub today: String,
    pub tomorrow: String,
    /// IANA name the page formats times in.
    pub time_zone: String,
}

pub fn local_days(now: DateTime<Utc>, tz: Tz) -> LocalDays {
    let today = now.with_timezone(&tz).date_naive();
    let start = |day: NaiveDate| format_utc(day_start(day, tz));
    LocalDays {
        yesterday: start(today - Days::new(1)),
        today: start(today),
        tomorrow: start(today + Days::new(1)),
        time_zone: tz.name().to_string(),
    }
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

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn days(yesterday: &str, today: &str, tomorrow: &str, time_zone: &str) -> LocalDays {
        LocalDays {
            yesterday: yesterday.to_string(),
            today: today.to_string(),
            tomorrow: tomorrow.to_string(),
            time_zone: time_zone.to_string(),
        }
    }

    #[test]
    fn a_day_starts_at_local_midnight_in_utc() {
        for (tz, day, expected) in [
            (
                chrono_tz::Europe::Berlin,
                date(2026, 11, 12),
                at(2026, 11, 11, 23, 0),
            ),
            (
                chrono_tz::Europe::Berlin,
                date(2026, 7, 1),
                at(2026, 6, 30, 22, 0),
            ),
            (
                chrono_tz::America::New_York,
                date(2026, 11, 12),
                at(2026, 11, 12, 5, 0),
            ),
            (chrono_tz::UTC, date(2026, 11, 12), at(2026, 11, 12, 0, 0)),
        ] {
            assert_eq!(day_start(day, tz), expected, "{} {day}", tz.name());
        }
    }

    #[test]
    fn a_day_whose_midnight_is_skipped_starts_at_the_first_local_time_that_exists() {
        // Sao Paulo moved its clocks from 00:00 to 01:00 (UTC-3 to UTC-2) on 2018-11-04.
        assert_eq!(
            day_start(date(2018, 11, 4), chrono_tz::America::Sao_Paulo),
            at(2018, 11, 4, 3, 0)
        );
    }

    #[test]
    fn local_days_on_a_normal_day_start_at_berlin_midnights() {
        assert_eq!(
            local_days(at(2026, 11, 12, 10, 0), chrono_tz::Europe::Berlin),
            days(
                "2026-11-10T23:00:00.000Z",
                "2026-11-11T23:00:00.000Z",
                "2026-11-12T23:00:00.000Z",
                "Europe/Berlin"
            )
        );
    }

    #[test]
    fn a_moment_after_local_midnight_but_before_utc_midnight_belongs_to_the_new_local_day() {
        assert_eq!(
            local_days(at(2026, 11, 12, 23, 30), chrono_tz::Europe::Berlin),
            days(
                "2026-11-11T23:00:00.000Z",
                "2026-11-12T23:00:00.000Z",
                "2026-11-13T23:00:00.000Z",
                "Europe/Berlin"
            )
        );
    }

    #[test]
    fn the_day_before_a_clock_change_ends_one_hour_later_in_berlin() {
        let after = local_days(at(2026, 10, 26, 10, 0), chrono_tz::Europe::Berlin);
        assert_eq!(
            after,
            days(
                "2026-10-24T22:00:00.000Z",
                "2026-10-25T23:00:00.000Z",
                "2026-10-26T23:00:00.000Z",
                "Europe/Berlin"
            )
        );
        let on = local_days(at(2026, 10, 25, 10, 0), chrono_tz::Europe::Berlin);
        assert_eq!(
            on,
            days(
                "2026-10-23T22:00:00.000Z",
                "2026-10-24T22:00:00.000Z",
                "2026-10-25T23:00:00.000Z",
                "Europe/Berlin"
            )
        );
    }

    #[test]
    fn local_days_in_utc_start_at_utc_midnights() {
        assert_eq!(
            local_days(at(2026, 11, 12, 10, 0), chrono_tz::UTC),
            days(
                "2026-11-11T00:00:00.000Z",
                "2026-11-12T00:00:00.000Z",
                "2026-11-13T00:00:00.000Z",
                "UTC"
            )
        );
    }

    #[test]
    fn local_days_serialize_with_camel_case_names() {
        let value =
            serde_json::to_value(local_days(at(2026, 11, 12, 10, 0), chrono_tz::UTC)).unwrap();
        assert_eq!(
            value,
            json!({
                "yesterday": "2026-11-11T00:00:00.000Z",
                "today": "2026-11-12T00:00:00.000Z",
                "tomorrow": "2026-11-13T00:00:00.000Z",
                "timeZone": "UTC",
            })
        );
    }

    #[test]
    fn something_that_is_not_a_jira_time_is_none() {
        for value in [json!(null), json!(42), json!("yesterday"), json!("")] {
            assert_eq!(utc(&value), None, "{value}");
        }
    }
}
