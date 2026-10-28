//! When the sync runs by itself: deltas every few minutes (shorter while the window has focus), a
//! reconciliation every hour, a full sync every week, and a delta right after the Mac wakes or the network
//! returns. `due` decides (pure); `start_schedule` ticks.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};

use super::{Job, SyncContext, SyncEngine, SyncState};
use crate::store::settings::SyncSettings;

const TICK: StdDuration = StdDuration::from_secs(30);
/// A tick that arrives much later than `TICK` means the Mac slept.
const WAKE_GAP: Duration = Duration::minutes(2);
const RECONCILE_EVERY: Duration = Duration::hours(1);
const FULL_EVERY: Duration = Duration::days(7);
/// While offline, a delta is tried this often, so the cache catches up soon after the network returns.
const OFFLINE_RETRY: Duration = Duration::minutes(1);

/// When each job was last queued.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Last {
    pub delta: DateTime<Utc>,
    pub reconcile: DateTime<Utc>,
    pub full: DateTime<Utc>,
    pub tick: DateTime<Utc>,
}

/// The jobs to queue at `now`, at most one: a full sync covers a reconciliation and a delta.
pub fn due(
    now: DateTime<Utc>,
    last: &Last,
    focused: bool,
    offline: bool,
    sync: &SyncSettings,
) -> Option<Job> {
    let woke = now - last.tick > WAKE_GAP;
    let every = if offline {
        OFFLINE_RETRY
    } else {
        Duration::minutes(i64::from(if focused {
            sync.focused_minutes
        } else {
            sync.background_minutes
        }))
    };
    if now - last.full >= FULL_EVERY {
        Some(Job::Full)
    } else if now - last.reconcile >= RECONCILE_EVERY {
        Some(Job::Reconcile)
    } else if woke || now - last.delta >= every {
        Some(Job::Delta)
    } else {
        None
    }
}

/// Moves the clocks of `job` (and of the jobs it includes) to `now`.
pub fn queued(last: &mut Last, job: Job, now: DateTime<Utc>) {
    match job {
        Job::Full => {
            last.full = now;
            last.reconcile = now;
            last.delta = now;
        }
        Job::Reconcile => last.reconcile = now,
        Job::Delta => last.delta = now,
    }
}

/// Whether the main window has focus; the window's focus events set it.
#[derive(Clone, Default)]
pub struct Focus(Arc<AtomicBool>);

impl Focus {
    pub fn set(&self, focused: bool) {
        self.0.store(focused, Ordering::Relaxed);
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Ticks for as long as the app runs and queues what is due on `engine`. The first deltas wait one interval:
/// the app start already synced or will (a never-synced cache runs a full sync at start).
pub fn start_schedule(ctx: SyncContext, engine: SyncEngine, focus: Focus) {
    tauri::async_runtime::spawn(async move {
        let now = Utc::now();
        let last_full = ctx
            .db
            .read(super::store::last_full_at)
            .await
            .ok()
            .flatten()
            .and_then(|at| DateTime::parse_from_rfc3339(&at).ok())
            .map_or(now, |at| at.with_timezone(&Utc));
        let mut last = Last {
            delta: now,
            reconcile: now,
            full: last_full,
            tick: now,
        };
        loop {
            tokio::time::sleep(TICK).await;
            let now = Utc::now();
            let offline = matches!(engine.state(), SyncState::Offline { .. });
            if let Some(job) = due(now, &last, focus.get(), offline, &ctx.settings.get().sync) {
                engine.queue(job);
                queued(&mut last, job, now);
            }
            last.tick = now;
        }
    });
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 28, 12, 0, 0).unwrap()
    }

    /// Every clock a few seconds old, the last tick 30 s ago: nothing is due.
    fn fresh() -> Last {
        Last {
            delta: now() - Duration::seconds(10),
            reconcile: now() - Duration::seconds(10),
            full: now() - Duration::seconds(10),
            tick: now() - Duration::seconds(30),
        }
    }

    fn due_now(last: &Last, focused: bool, offline: bool) -> Option<Job> {
        due(now(), last, focused, offline, &SyncSettings::default())
    }

    #[test]
    fn nothing_is_due_right_after_a_sync() {
        assert_eq!(due_now(&fresh(), true, false), None);
        assert_eq!(due_now(&fresh(), false, false), None);
    }

    #[test]
    fn a_full_sync_is_due_after_seven_days() {
        for (age, expected) in [
            (Duration::days(7) - Duration::seconds(1), None),
            (Duration::days(7), Some(Job::Full)),
            (Duration::days(30), Some(Job::Full)),
        ] {
            let last = Last {
                full: now() - age,
                ..fresh()
            };
            assert_eq!(due_now(&last, true, false), expected, "{age:?}");
        }
    }

    #[test]
    fn a_full_sync_wins_over_a_due_reconcile_and_delta() {
        let last = Last {
            delta: now() - Duration::hours(3),
            reconcile: now() - Duration::hours(3),
            full: now() - Duration::days(8),
            tick: now() - Duration::minutes(10),
        };

        assert_eq!(due_now(&last, true, false), Some(Job::Full));
    }

    #[test]
    fn a_reconcile_is_due_after_an_hour() {
        for (age, expected) in [
            (Duration::minutes(59), None),
            (Duration::hours(1), Some(Job::Reconcile)),
            (Duration::hours(5), Some(Job::Reconcile)),
        ] {
            let last = Last {
                reconcile: now() - age,
                ..fresh()
            };
            assert_eq!(due_now(&last, true, false), expected, "{age:?}");
        }
    }

    #[test]
    fn a_reconcile_wins_over_a_due_delta() {
        let last = Last {
            delta: now() - Duration::hours(2),
            reconcile: now() - Duration::hours(2),
            ..fresh()
        };

        assert_eq!(due_now(&last, false, false), Some(Job::Reconcile));
    }

    #[test]
    fn a_delta_waits_for_the_focused_or_the_background_interval() {
        for (focused, minutes, expected) in [
            (true, 4, None),
            (true, 5, Some(Job::Delta)),
            (true, 14, Some(Job::Delta)),
            (false, 5, None),
            (false, 14, None),
            (false, 15, Some(Job::Delta)),
        ] {
            let last = Last {
                delta: now() - Duration::minutes(minutes),
                ..fresh()
            };
            assert_eq!(
                due_now(&last, focused, false),
                expected,
                "focused {focused}, {minutes} min"
            );
        }
    }

    #[test]
    fn the_intervals_come_from_the_settings() {
        let sync = SyncSettings {
            focused_minutes: 2,
            background_minutes: 30,
            ..SyncSettings::default()
        };
        let last = Last {
            delta: now() - Duration::minutes(3),
            ..fresh()
        };

        assert_eq!(due(now(), &last, true, false, &sync), Some(Job::Delta));
        assert_eq!(due(now(), &last, false, false, &sync), None);
    }

    #[test]
    fn a_delta_is_due_when_the_mac_woke_up() {
        for (gap, expected) in [
            (Duration::minutes(2), None),
            (
                Duration::minutes(2) + Duration::seconds(1),
                Some(Job::Delta),
            ),
            (Duration::hours(8), Some(Job::Delta)),
        ] {
            let last = Last {
                tick: now() - gap,
                ..fresh()
            };
            assert_eq!(due_now(&last, false, false), expected, "{gap:?}");
        }
    }

    #[test]
    fn while_offline_a_delta_is_retried_every_minute() {
        for (age, offline, expected) in [
            (Duration::seconds(59), true, None),
            (Duration::minutes(1), true, Some(Job::Delta)),
            (Duration::minutes(1), false, None),
        ] {
            let last = Last {
                delta: now() - age,
                ..fresh()
            };
            assert_eq!(
                due_now(&last, false, offline),
                expected,
                "{age:?}, offline {offline}"
            );
        }
    }

    #[test]
    fn a_full_sync_covers_the_reconcile_and_the_delta() {
        let old = now() - Duration::days(9);
        let mut last = Last {
            delta: old,
            reconcile: old,
            full: old,
            tick: old,
        };

        queued(&mut last, Job::Full, now());

        assert_eq!(
            last,
            Last {
                delta: now(),
                reconcile: now(),
                full: now(),
                tick: old,
            }
        );
    }

    #[test]
    fn a_reconcile_and_a_delta_move_only_their_own_clock() {
        let old = now() - Duration::days(1);
        let start = Last {
            delta: old,
            reconcile: old,
            full: old,
            tick: old,
        };

        let mut after_reconcile = start;
        queued(&mut after_reconcile, Job::Reconcile, now());
        let mut after_delta = start;
        queued(&mut after_delta, Job::Delta, now());

        assert_eq!(
            after_reconcile,
            Last {
                reconcile: now(),
                ..start
            }
        );
        assert_eq!(
            after_delta,
            Last {
                delta: now(),
                ..start
            }
        );
    }

    #[test]
    fn nothing_is_due_again_right_after_the_job_was_queued() {
        let mut last = Last {
            full: now() - Duration::days(8),
            ..fresh()
        };
        assert_eq!(due_now(&last, true, false), Some(Job::Full));

        queued(&mut last, Job::Full, now());

        assert_eq!(due_now(&last, true, false), None);
    }

    #[test]
    fn the_focus_starts_unfocused_and_is_shared_between_clones() {
        let focus = Focus::default();
        let clone = focus.clone();
        assert!(!focus.get());

        clone.set(true);
        assert!(focus.get());

        clone.set(false);
        assert!(!focus.get());
    }
}
