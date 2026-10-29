//! Turns the tables' Berlin wall-clock times into Jira timestamps, shifted to a chosen "now".

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeDelta, TimeZone, Utc};
use chrono_tz::Europe::Berlin;

use super::DEMO_NOW;

const LOCAL_FORMAT: &str = "%Y-%m-%d %H:%M";
const DATE_FORMAT: &str = "%Y-%m-%d";

pub struct Clock {
    shift: TimeDelta,
    day_shift: i64,
}

impl Clock {
    /// A clock that keeps every table time as far before `now` as it is before [`DEMO_NOW`].
    pub fn new(now: DateTime<Utc>) -> Self {
        let anchor = demo_now();
        let local_day = |at: DateTime<Utc>| at.with_timezone(&Berlin).date_naive();
        Self {
            shift: now - anchor,
            day_shift: (local_day(now) - local_day(anchor)).num_days(),
        }
    }

    /// `"2026-10-28 11:42"` (Berlin) as Jira's `2026-10-28T11:42:00.000+0100`, shifted exactly.
    pub fn stamp(&self, local: &str) -> String {
        let at = vienna(parse(local)) + self.shift;
        at.with_timezone(&Berlin)
            .format("%Y-%m-%dT%H:%M:%S%.3f%z")
            .to_string()
    }

    /// A planned time (sprint start and end) in UTC as the agile API writes it, shifted by whole
    /// days so it keeps its time of day.
    pub fn planned_utc(&self, local: &str) -> String {
        vienna(parse(local) + self.days())
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string()
    }

    /// A calendar date (due dates, release dates), shifted by whole days.
    pub fn date(&self, date: &str) -> String {
        let parsed = NaiveDate::parse_from_str(date, DATE_FORMAT).expect("demo date");
        (parsed + self.days()).format(DATE_FORMAT).to_string()
    }

    fn days(&self) -> TimeDelta {
        TimeDelta::days(self.day_shift)
    }
}

fn parse(local: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(local, LOCAL_FORMAT).expect("demo time")
}

fn vienna(naive: NaiveDateTime) -> DateTime<Utc> {
    Berlin
        .from_local_datetime(&naive)
        .earliest()
        .expect("demo time in Berlin")
        .with_timezone(&Utc)
}

pub fn demo_now() -> DateTime<Utc> {
    DEMO_NOW.parse().expect("DEMO_NOW is RFC 3339")
}

/// Formats an instant as the Berlin wall-clock time the tables use.
pub fn local(at: DateTime<Utc>) -> String {
    at.with_timezone(&Berlin).format(LOCAL_FORMAT).to_string()
}
