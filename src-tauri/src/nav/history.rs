//! A tab's Back and Forward (ADR 0052): the places and issues it showed, newest last.

use serde::{Deserialize, Serialize};

use super::Location;

/// Steps kept per direction; older ones are dropped.
pub const KEPT: usize = 100;

/// Where a tab was: a level's view, and the issue its Issue view showed there.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub location: Location,
    #[serde(default)]
    pub issue: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct History {
    back: Vec<Step>,
    forward: Vec<Step>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Back,
    Forward,
}

impl History {
    /// The tab left `from` for a new step: Back returns to it, and nothing lies ahead anymore.
    pub fn left(&mut self, from: Step) {
        if self.back.last() != Some(&from) {
            self.back.push(from);
        }
        let over = self.back.len().saturating_sub(KEPT);
        self.back.drain(..over);
        self.forward.clear();
    }

    pub fn can(&self, direction: Direction) -> bool {
        !self.stack(direction).is_empty()
    }

    /// Takes the next step in `direction`, without noting where the tab is (`arrived`).
    pub fn take(&mut self, direction: Direction) -> Option<Step> {
        match direction {
            Direction::Back => self.back.pop(),
            Direction::Forward => self.forward.pop(),
        }
    }

    /// The tab moved from `here` in `direction`: going the other way returns to it.
    pub fn arrived(&mut self, direction: Direction, here: Step) {
        let other = match direction {
            Direction::Back => &mut self.forward,
            Direction::Forward => &mut self.back,
        };
        other.push(here);
        let over = other.len().saturating_sub(KEPT);
        other.drain(..over);
    }

    fn stack(&self, direction: Direction) -> &[Step] {
        match direction {
            Direction::Back => &self.back,
            Direction::Forward => &self.forward,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(list_id: i64) -> Step {
        Step {
            location: Location {
                list_id,
                view_id: list_id * 10,
            },
            issue: None,
        }
    }

    fn with_issue(list_id: i64, issue: &str) -> Step {
        Step {
            issue: Some(issue.into()),
            ..step(list_id)
        }
    }

    fn drain(history: &mut History, direction: Direction) -> Vec<Step> {
        std::iter::from_fn(|| history.take(direction)).collect()
    }

    #[test]
    fn a_new_history_can_go_nowhere() {
        let mut history = History::default();

        for direction in [Direction::Back, Direction::Forward] {
            assert!(!history.can(direction), "{direction:?}");
            assert_eq!(history.take(direction), None, "{direction:?}");
        }
    }

    #[test]
    fn a_left_step_can_be_gone_back_to() {
        let mut history = History::default();

        history.left(step(1));

        assert!(history.can(Direction::Back));
        assert!(!history.can(Direction::Forward));
        assert_eq!(history.take(Direction::Back), Some(step(1)));
        assert!(!history.can(Direction::Back));
    }

    #[test]
    fn back_takes_the_newest_step_first() {
        let mut history = History::default();
        history.left(step(1));
        history.left(step(2));
        history.left(step(3));

        assert_eq!(
            drain(&mut history, Direction::Back),
            [step(3), step(2), step(1)]
        );
    }

    #[test]
    fn going_back_and_forward_again_round_trips() {
        let mut history = History::default();
        history.left(step(1));

        let back = history.take(Direction::Back).unwrap();
        history.arrived(Direction::Back, step(2));
        assert_eq!(back, step(1));
        assert!(history.can(Direction::Forward));
        assert!(!history.can(Direction::Back));

        let forward = history.take(Direction::Forward).unwrap();
        history.arrived(Direction::Forward, step(1));
        assert_eq!(forward, step(2));
        assert!(history.can(Direction::Back));
        assert!(!history.can(Direction::Forward));
        assert_eq!(history.take(Direction::Back), Some(step(1)));
    }

    #[test]
    fn a_new_step_clears_forward() {
        let mut history = History::default();
        history.left(step(1));
        history.take(Direction::Back);
        history.arrived(Direction::Back, step(2));
        assert!(history.can(Direction::Forward));

        history.left(step(3));

        assert!(!history.can(Direction::Forward));
        assert_eq!(drain(&mut history, Direction::Back), [step(3)]);
    }

    #[test]
    fn leaving_the_same_step_twice_is_noted_once_but_still_clears_forward() {
        let mut history = History::default();
        history.left(step(1));
        history.arrived(Direction::Back, step(5));
        assert!(history.can(Direction::Forward));

        history.left(step(1));

        assert!(!history.can(Direction::Forward));
        assert_eq!(drain(&mut history, Direction::Back), [step(1)]);
    }

    #[test]
    fn the_same_place_with_another_issue_is_a_different_step() {
        let mut history = History::default();

        history.left(step(1));
        history.left(with_issue(1, "NW-1"));
        history.left(with_issue(1, "NW-1"));

        assert_eq!(
            drain(&mut history, Direction::Back),
            [with_issue(1, "NW-1"), step(1)]
        );
    }

    #[test]
    fn only_the_newest_hundred_steps_are_kept_going_back() {
        let mut history = History::default();
        for n in 0..150 {
            history.left(step(n));
        }

        let kept = drain(&mut history, Direction::Back);

        assert_eq!(kept.len(), KEPT);
        assert_eq!(kept.first(), Some(&step(149)));
        assert_eq!(kept.last(), Some(&step(50)));
    }

    #[test]
    fn only_the_newest_hundred_steps_are_kept_going_forward() {
        let mut history = History::default();
        for n in 0..150 {
            history.arrived(Direction::Back, step(n));
        }

        let kept = drain(&mut history, Direction::Forward);

        assert_eq!(kept.len(), KEPT);
        assert_eq!(kept.first(), Some(&step(149)));
        assert_eq!(kept.last(), Some(&step(50)));
    }

    #[test]
    fn arriving_going_forward_fills_back_up_to_the_cap() {
        let mut history = History::default();
        for n in 0..120 {
            history.arrived(Direction::Forward, step(n));
        }

        assert_eq!(drain(&mut history, Direction::Back).len(), KEPT);
    }

    #[test]
    fn a_step_serializes_with_its_location_and_issue() {
        assert_eq!(
            serde_json::to_value(with_issue(1, "NW-1")).unwrap(),
            serde_json::json!({
                "location": { "listId": 1, "viewId": 10 },
                "issue": "NW-1",
            })
        );
    }

    #[test]
    fn a_stored_history_without_stacks_loads_empty() {
        let history: History = serde_json::from_value(serde_json::json!({})).unwrap();

        assert_eq!(history, History::default());
    }
}
