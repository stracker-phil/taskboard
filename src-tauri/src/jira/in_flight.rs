//! What Taskboard is asking Jira right now, in words, for the status bar (ADR 0060).

use std::sync::{Arc, Mutex};

use serde::Serialize;
use ts_rs::TS;

use crate::lock::MutexExt;

#[derive(Serialize, Clone, Debug, Default, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct JiraActivity {
    /// One entry per request in flight, oldest first ("Saving the status").
    pub doing: Vec<String>,
}

pub type Notifier = Box<dyn Fn(&JiraActivity) + Send + Sync>;

#[derive(Default)]
pub struct InFlight {
    requests: Mutex<Requests>,
    notifier: Mutex<Option<Notifier>>,
}

#[derive(Default)]
struct Requests {
    next_id: u64,
    running: Vec<(u64, String)>,
}

impl InFlight {
    /// Called with the whole activity after every request starts or ends.
    pub fn set_notifier(&self, notifier: Notifier) {
        *self.notifier.lock_unpoisoned() = Some(notifier);
    }

    /// Counts a request until the returned guard drops.
    pub fn begin(self: &Arc<Self>, doing: &str) -> Running {
        let id = {
            let mut requests = self.requests.lock_unpoisoned();
            requests.next_id += 1;
            let id = requests.next_id;
            requests.running.push((id, doing.into()));
            id
        };
        self.announce();
        Running {
            owner: Arc::clone(self),
            id,
        }
    }

    pub fn activity(&self) -> JiraActivity {
        let requests = self.requests.lock_unpoisoned();
        JiraActivity {
            doing: requests
                .running
                .iter()
                .map(|(_, doing)| doing.clone())
                .collect(),
        }
    }

    fn end(&self, id: u64) {
        self.requests
            .lock_unpoisoned()
            .running
            .retain(|(running, _)| *running != id);
        self.announce();
    }

    fn announce(&self) {
        let activity = self.activity();
        if let Some(notify) = self.notifier.lock_unpoisoned().as_ref() {
            notify(&activity);
        }
    }
}

/// A request in flight; dropping it, also on an early return or a panic, ends it.
pub struct Running {
    owner: Arc<InFlight>,
    id: u64,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.owner.end(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doing(items: &[&str]) -> JiraActivity {
        JiraActivity {
            doing: items.iter().map(|item| item.to_string()).collect(),
        }
    }

    #[test]
    fn activity_lists_the_requests_oldest_first() {
        let in_flight = Arc::new(InFlight::default());

        let _first = in_flight.begin("Loading the issues");
        let _second = in_flight.begin("Saving the status");
        let _third = in_flight.begin("Sending the comment");

        assert_eq!(
            in_flight.activity(),
            doing(&[
                "Loading the issues",
                "Saving the status",
                "Sending the comment"
            ])
        );
    }

    #[test]
    fn a_dropped_guard_ends_only_its_own_request() {
        let in_flight = Arc::new(InFlight::default());
        let first = in_flight.begin("Loading the issues");
        let _second = in_flight.begin("Saving the status");

        drop(first);

        assert_eq!(in_flight.activity(), doing(&["Saving the status"]));
    }

    #[test]
    fn two_requests_with_the_same_words_end_separately() {
        let in_flight = Arc::new(InFlight::default());
        let first = in_flight.begin("Loading the issues");
        let second = in_flight.begin("Loading the issues");

        drop(second);
        assert_eq!(in_flight.activity(), doing(&["Loading the issues"]));

        drop(first);
        assert_eq!(in_flight.activity(), doing(&[]));
    }

    #[test]
    fn the_notifier_hears_the_whole_activity_after_each_start_and_end() {
        let in_flight = Arc::new(InFlight::default());
        let heard = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&heard);
        in_flight.set_notifier(Box::new(move |activity| {
            sink.lock().unwrap().push(activity.clone());
        }));

        let first = in_flight.begin("Loading the issues");
        let second = in_flight.begin("Saving the status");
        drop(first);
        drop(second);

        assert_eq!(
            *heard.lock().unwrap(),
            [
                doing(&["Loading the issues"]),
                doing(&["Loading the issues", "Saving the status"]),
                doing(&["Saving the status"]),
                doing(&[]),
            ]
        );
    }

    #[test]
    fn the_activity_is_empty_before_anything_starts_and_after_everything_ends() {
        let in_flight = Arc::new(InFlight::default());
        assert_eq!(in_flight.activity(), JiraActivity::default());

        let guards = [in_flight.begin("a"), in_flight.begin("b")];
        drop(guards);

        assert_eq!(in_flight.activity(), JiraActivity::default());
    }

    #[test]
    fn the_activity_serializes_with_a_doing_list() {
        let value = serde_json::to_value(doing(&["Saving the status"])).unwrap();

        assert_eq!(value, serde_json::json!({ "doing": ["Saving the status"] }));
    }
}
