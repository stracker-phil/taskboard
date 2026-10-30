//! Announcing changes at most once per window: a sync stores a page every few milliseconds, and each
//! announcement makes the page refetch.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::{Changes, Notifier};
use crate::lock::MutexExt;

/// The least time between two announcements.
pub const ANNOUNCE_EVERY: Duration = Duration::from_millis(500);

#[derive(Default)]
struct Pending {
    changes: Option<Changes>,
    last: Option<Instant>,
}

/// Passes the first change on at once; changes within `window` of the last announcement are merged and
/// announced together when it ends, so the last one is never lost.
pub fn coalesce(window: Duration, notify: Notifier) -> Notifier {
    let notify: Arc<Notifier> = Arc::new(notify);
    let pending = Arc::new(Mutex::new(Pending::default()));
    Box::new(move |changes| {
        let mut state = pending.lock_unpoisoned();
        if let Some(waiting) = &mut state.changes {
            waiting.merge(changes.clone());
            return;
        }
        let now = Instant::now();
        match state
            .last
            .map(|last| last + window)
            .filter(|due| *due > now)
        {
            None => {
                state.last = Some(now);
                drop(state);
                notify(changes);
            }
            Some(due) => {
                state.changes = Some(changes.clone());
                drop(state);
                flush_at(due, Arc::clone(&pending), Arc::clone(&notify));
            }
        }
    })
}

/// Announces what has gathered in `pending` once `due` has passed.
fn flush_at(due: Instant, pending: Arc<Mutex<Pending>>, notify: Arc<Notifier>) {
    thread::spawn(move || {
        thread::sleep(due.saturating_duration_since(Instant::now()));
        let mut state = pending.lock_unpoisoned();
        let changes = state.changes.take();
        state.last = Some(Instant::now());
        drop(state);
        if let Some(changes) = changes {
            notify(&changes);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const WINDOW: Duration = Duration::from_millis(100);
    const WELL_PAST_WINDOW: Duration = Duration::from_millis(400);

    type Sink = Arc<Mutex<Vec<Changes>>>;

    fn coalescing() -> (Notifier, Sink) {
        let sink: Sink = Arc::default();
        let into = Arc::clone(&sink);
        let notifier = coalesce(
            WINDOW,
            Box::new(move |changes| into.lock().unwrap().push(changes.clone())),
        );
        (notifier, sink)
    }

    fn issue(id: &str) -> Changes {
        let mut changes = Changes::default();
        changes.issue_changed(id);
        changes
    }

    fn announced(sink: &Sink) -> Vec<Changes> {
        sink.lock().unwrap().clone()
    }

    #[test]
    fn a_single_change_is_announced_at_once() {
        let (notify, sink) = coalescing();

        notify(&issue("20001"));

        assert_eq!(announced(&sink), [issue("20001")]);
    }

    #[test]
    fn a_burst_announces_the_first_at_once_and_the_rest_merged_once_later() {
        let (notify, sink) = coalescing();

        for id in ["20001", "20002", "20003", "20004"] {
            notify(&issue(id));
        }

        assert_eq!(announced(&sink), [issue("20001")]);
        thread::sleep(WELL_PAST_WINDOW);
        assert_eq!(
            announced(&sink),
            [
                issue("20001"),
                Changes {
                    issues: BTreeSet::from([
                        "20002".to_string(),
                        "20003".to_string(),
                        "20004".to_string()
                    ]),
                    ..Changes::default()
                }
            ]
        );
        thread::sleep(WELL_PAST_WINDOW);
        assert_eq!(announced(&sink).len(), 2);
    }

    #[test]
    fn a_change_after_a_quiet_period_is_announced_at_once_again() {
        let (notify, sink) = coalescing();
        notify(&issue("20001"));
        thread::sleep(WELL_PAST_WINDOW);

        notify(&issue("20002"));

        assert_eq!(announced(&sink), [issue("20001"), issue("20002")]);
    }

    #[test]
    fn a_second_burst_after_a_quiet_period_is_coalesced_like_the_first() {
        let (notify, sink) = coalescing();
        notify(&issue("20001"));
        notify(&issue("20002"));
        thread::sleep(WELL_PAST_WINDOW);
        assert_eq!(announced(&sink), [issue("20001"), issue("20002")]);

        notify(&issue("20003"));
        notify(&issue("20004"));
        assert_eq!(
            announced(&sink),
            [issue("20001"), issue("20002"), issue("20003")]
        );
        thread::sleep(WELL_PAST_WINDOW);

        assert_eq!(
            announced(&sink),
            [
                issue("20001"),
                issue("20002"),
                issue("20003"),
                issue("20004")
            ]
        );
    }

    #[test]
    fn every_kind_of_change_in_a_burst_survives_the_merge() {
        let (notify, sink) = coalescing();
        notify(&issue("20001"));
        let mut detail = Changes::default();
        detail.detail_changed("20002");
        notify(&detail);
        let mut local = Changes::default();
        local.local_changed(crate::db::LocalPart::Pins);
        notify(&local);

        thread::sleep(WELL_PAST_WINDOW);

        assert_eq!(announced(&sink).len(), 2);
        assert_eq!(
            announced(&sink)[1],
            Changes {
                details: BTreeSet::from(["20002".to_string()]),
                local: BTreeSet::from([crate::db::LocalPart::Pins]),
                ..Changes::default()
            }
        );
    }
}
