//! How long a failing sync waits before it tries again. Offline costs nothing to retry, so it retries every minute;
//! a failed job waits longer after each failure in a row, up to a cap.

use chrono::{DateTime, Duration, Utc};

use super::SyncState;
use crate::time::format_utc;

const OFFLINE_RETRY: Duration = Duration::minutes(1);
const FIRST_RETRY: Duration = Duration::minutes(1);
const LONGEST_RETRY: Duration = Duration::minutes(15);

/// The failures in a row since the last job that worked.
#[derive(Debug, Default)]
pub struct Failures {
    count: u32,
    since: Option<DateTime<Utc>>,
}

impl Failures {
    /// `next` with the streak's first failure as `since` and the time of the next try as `retry_at`; any other state
    /// ends the streak.
    pub fn follow(&mut self, next: SyncState, now: DateTime<Utc>) -> SyncState {
        match next {
            SyncState::Error {
                kind,
                message,
                last_ok,
                ..
            } => {
                let (since, retry_at) = self.failed(now, retry_delay(self.count));
                SyncState::Error {
                    kind,
                    message,
                    since,
                    retry_at,
                    last_ok,
                }
            }
            SyncState::Offline { last_ok, .. } => {
                let (since, retry_at) = self.failed(now, OFFLINE_RETRY);
                SyncState::Offline {
                    since,
                    retry_at,
                    last_ok,
                }
            }
            other => {
                *self = Self::default();
                other
            }
        }
    }

    fn failed(&mut self, now: DateTime<Utc>, wait: Duration) -> (String, String) {
        self.count += 1;
        let since = *self.since.get_or_insert(now);
        (format_utc(since), format_utc(now + wait))
    }
}

/// 1, 2, 4, 8 minutes, then 15 minutes for every further try.
fn retry_delay(failed_before: u32) -> Duration {
    let doubled = FIRST_RETRY * 2_i32.saturating_pow(failed_before.min(8));
    doubled.min(LONGEST_RETRY)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::error::ErrorKind;

    const LAST_OK: &str = "2026-10-28T11:00:00.000Z";

    fn at(minutes: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap() + Duration::minutes(minutes)
    }

    fn stamp(minutes: i64) -> String {
        format_utc(at(minutes))
    }

    fn error() -> SyncState {
        SyncState::Error {
            kind: ErrorKind::Internal,
            message: "Jira answered 500".into(),
            since: String::new(),
            retry_at: String::new(),
            last_ok: Some(LAST_OK.into()),
        }
    }

    fn offline() -> SyncState {
        SyncState::Offline {
            since: String::new(),
            retry_at: String::new(),
            last_ok: Some(LAST_OK.into()),
        }
    }

    fn failed_error(since: i64, retry: i64) -> SyncState {
        SyncState::Error {
            kind: ErrorKind::Internal,
            message: "Jira answered 500".into(),
            since: stamp(since),
            retry_at: stamp(retry),
            last_ok: Some(LAST_OK.into()),
        }
    }

    fn failed_offline(since: i64, retry: i64) -> SyncState {
        SyncState::Offline {
            since: stamp(since),
            retry_at: stamp(retry),
            last_ok: Some(LAST_OK.into()),
        }
    }

    #[test]
    fn the_first_error_retries_after_one_minute_and_starts_the_streak_now() {
        let mut failures = Failures::default();

        assert_eq!(failures.follow(error(), at(0)), failed_error(0, 1));
    }

    #[test]
    fn errors_in_a_row_wait_one_two_four_eight_then_fifteen_minutes() {
        let mut failures = Failures::default();
        let mut now = 0;

        for (n, wait) in [1, 2, 4, 8, 15, 15, 15, 15, 15, 15].into_iter().enumerate() {
            assert_eq!(
                failures.follow(error(), at(now)),
                failed_error(0, now + wait),
                "failure {}",
                n + 1
            );
            now += wait;
        }
    }

    #[test]
    fn offline_retries_every_minute_however_long_it_lasts() {
        let mut failures = Failures::default();

        for minute in 0..6 {
            assert_eq!(
                failures.follow(offline(), at(minute)),
                failed_offline(0, minute + 1),
                "minute {minute}"
            );
        }
    }

    #[test]
    fn offline_and_errors_share_one_streak() {
        let mut failures = Failures::default();

        assert_eq!(failures.follow(offline(), at(0)), failed_offline(0, 1));
        assert_eq!(failures.follow(error(), at(1)), failed_error(0, 3));
        assert_eq!(failures.follow(error(), at(3)), failed_error(0, 7));
        assert_eq!(failures.follow(offline(), at(7)), failed_offline(0, 8));
        assert_eq!(failures.follow(error(), at(8)), failed_error(0, 23));
    }

    #[test]
    fn a_job_that_worked_ends_the_streak_and_passes_through() {
        let mut failures = Failures::default();
        failures.follow(error(), at(0));
        failures.follow(error(), at(1));

        let idle = SyncState::Idle {
            last_ok: Some(stamp(5)),
        };
        assert_eq!(failures.follow(idle.clone(), at(5)), idle);

        assert_eq!(failures.follow(error(), at(10)), failed_error(10, 11));
    }

    #[test]
    fn a_paused_sync_ends_the_streak_and_passes_through() {
        let mut failures = Failures::default();
        failures.follow(offline(), at(0));

        let paused = SyncState::Paused {
            message: "Jira refused the credentials".into(),
        };
        assert_eq!(failures.follow(paused.clone(), at(2)), paused);

        assert_eq!(failures.follow(offline(), at(4)), failed_offline(4, 5));
    }

    #[test]
    fn the_failure_keeps_its_kind_message_and_last_success() {
        let mut failures = Failures::default();
        let state = SyncState::Error {
            kind: ErrorKind::NotFound,
            message: "Project NW is gone".into(),
            since: String::new(),
            retry_at: String::new(),
            last_ok: None,
        };

        assert_eq!(
            failures.follow(state, at(0)),
            SyncState::Error {
                kind: ErrorKind::NotFound,
                message: "Project NW is gone".into(),
                since: stamp(0),
                retry_at: stamp(1),
                last_ok: None,
            }
        );
    }
}
