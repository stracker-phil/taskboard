//! How long a pin lasts: day-based durations end at the owner's local midnight.

use chrono::{DateTime, Days, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum PinDuration {
    Today,
    TwoDays,
    Week,
    /// Also what an unknown value in the settings file reads as.
    #[serde(other)]
    UntilUnpinned,
}

/// Today ends tonight, 2 days at the end of tomorrow, 1 week seven midnights on; none for Until unpinned.
pub fn expires_at(duration: PinDuration, now: DateTime<Utc>, tz: Tz) -> Option<DateTime<Utc>> {
    let days = match duration {
        PinDuration::Today => 1,
        PinDuration::TwoDays => 2,
        PinDuration::Week => 7,
        PinDuration::UntilUnpinned => return None,
    };
    let today = now.with_timezone(&tz).date_naive();
    Some(day_start(today + Days::new(days), tz))
}

/// Local midnight; on a day whose midnight a clock change skips, the first local time that exists.
fn day_start(date: NaiveDate, tz: Tz) -> DateTime<Utc> {
    let midnight = date.and_time(NaiveTime::MIN);
    (0..=3)
        .find_map(|hours| {
            tz.from_local_datetime(&(midnight + Duration::hours(hours)))
                .earliest()
        })
        .map_or_else(|| midnight.and_utc(), |at| at.with_timezone(&Utc))
}

/// "today" on its last day, "2d left" before; empty for a pin without an end.
pub fn time_left(expires: Option<DateTime<Utc>>, now: DateTime<Utc>, tz: Tz) -> String {
    let Some(expires) = expires else {
        return String::new();
    };
    let today = now.with_timezone(&tz).date_naive();
    let last_day = (expires - Duration::seconds(1))
        .with_timezone(&tz)
        .date_naive();
    match (last_day - today).num_days() {
        ..=0 => "today".into(),
        days => format!("{}d left", days + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(text: &str) -> DateTime<Utc> {
        text.parse().unwrap()
    }

    fn vienna() -> Tz {
        "Europe/Vienna".parse().unwrap()
    }

    fn ends(duration: PinDuration, now: &str, tz: Tz) -> Option<DateTime<Utc>> {
        expires_at(duration, utc(now), tz)
    }

    fn left(duration: PinDuration, created: &str, viewed: &str) -> String {
        let tz = vienna();
        time_left(expires_at(duration, utc(created), tz), utc(viewed), tz)
    }

    #[test]
    fn day_based_durations_end_at_a_later_local_midnight() {
        let now = "2026-10-28T10:00:00Z";
        for (duration, expected) in [
            (PinDuration::Today, "2026-10-28T23:00:00Z"),
            (PinDuration::TwoDays, "2026-10-29T23:00:00Z"),
            (PinDuration::Week, "2026-11-03T23:00:00Z"),
        ] {
            assert_eq!(
                ends(duration, now, vienna()),
                Some(utc(expected)),
                "{duration:?}"
            );
        }
    }

    #[test]
    fn until_unpinned_never_ends() {
        assert_eq!(
            ends(PinDuration::UntilUnpinned, "2026-10-28T10:00:00Z", vienna()),
            None
        );
    }

    #[test]
    fn today_ends_at_the_next_local_midnight_just_before_and_just_after_midnight() {
        for (now, expected) in [
            ("2026-10-28T22:30:00Z", "2026-10-28T23:00:00Z"),
            ("2026-10-28T23:10:00Z", "2026-10-29T23:00:00Z"),
        ] {
            assert_eq!(
                ends(PinDuration::Today, now, vienna()),
                Some(utc(expected)),
                "{now}"
            );
        }
    }

    #[test]
    fn the_zone_decides_which_day_it_is() {
        let now = "2026-10-28T23:30:00Z";

        assert_eq!(
            ends(PinDuration::Today, now, Tz::UTC),
            Some(utc("2026-10-29T00:00:00Z"))
        );
        assert_eq!(
            ends(PinDuration::Today, now, vienna()),
            Some(utc("2026-10-29T23:00:00Z"))
        );
    }

    #[test]
    fn durations_still_land_on_local_midnight_across_the_clocks_going_back() {
        for (duration, now, expected) in [
            (
                PinDuration::Today,
                "2026-10-24T12:00:00Z",
                "2026-10-24T22:00:00Z",
            ),
            (
                PinDuration::TwoDays,
                "2026-10-24T12:00:00Z",
                "2026-10-25T23:00:00Z",
            ),
            (
                PinDuration::Week,
                "2026-10-24T12:00:00Z",
                "2026-10-30T23:00:00Z",
            ),
            (
                PinDuration::Today,
                "2026-10-25T12:00:00Z",
                "2026-10-25T23:00:00Z",
            ),
        ] {
            assert_eq!(
                ends(duration, now, vienna()),
                Some(utc(expected)),
                "{duration:?} from {now}"
            );
        }
    }

    #[test]
    fn durations_still_land_on_local_midnight_across_the_clocks_going_forward() {
        for (duration, now, expected) in [
            (
                PinDuration::Today,
                "2026-03-28T12:00:00Z",
                "2026-03-28T23:00:00Z",
            ),
            (
                PinDuration::TwoDays,
                "2026-03-28T12:00:00Z",
                "2026-03-29T22:00:00Z",
            ),
            (
                PinDuration::Week,
                "2026-03-25T12:00:00Z",
                "2026-03-31T22:00:00Z",
            ),
        ] {
            assert_eq!(
                ends(duration, now, vienna()),
                Some(utc(expected)),
                "{duration:?} from {now}"
            );
        }
    }

    #[test]
    fn a_midnight_skipped_by_a_clock_change_ends_at_the_first_local_time_that_exists() {
        for (zone, now, expected) in [
            (
                "America/Santiago",
                "2026-09-05T15:00:00Z",
                "2026-09-06T04:00:00Z",
            ),
            (
                "America/Havana",
                "2026-03-07T15:00:00Z",
                "2026-03-08T05:00:00Z",
            ),
        ] {
            assert_eq!(
                ends(PinDuration::Today, now, zone.parse().unwrap()),
                Some(utc(expected)),
                "{zone}"
            );
        }
    }

    #[test]
    fn a_pin_without_an_end_has_no_time_left() {
        assert_eq!(time_left(None, utc("2026-10-28T10:00:00Z"), vienna()), "");
    }

    #[test]
    fn time_left_counts_the_days_until_the_last_day() {
        let created = "2026-10-28T10:00:00Z";
        for (duration, viewed, expected) in [
            (PinDuration::Today, created, "today"),
            (PinDuration::TwoDays, created, "2d left"),
            (PinDuration::TwoDays, "2026-10-29T10:00:00Z", "today"),
            (PinDuration::Week, created, "7d left"),
            (PinDuration::Week, "2026-10-29T10:00:00Z", "6d left"),
            (PinDuration::Week, "2026-11-03T10:00:00Z", "today"),
        ] {
            assert_eq!(
                left(duration, created, viewed),
                expected,
                "{duration:?} viewed {viewed}"
            );
        }
    }

    #[test]
    fn time_left_follows_the_local_day_not_the_utc_day() {
        assert_eq!(
            left(
                PinDuration::TwoDays,
                "2026-10-28T10:00:00Z",
                "2026-10-28T23:30:00Z"
            ),
            "today"
        );
    }
}
