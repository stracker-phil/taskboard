//! What a write changed, announced to the page once the transaction committed.

use std::collections::BTreeSet;

use serde::Serialize;
use ts_rs::TS;

/// A part of the owner's own setup (the local file).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum LocalPart {
    Spaces,
    Lists,
    Views,
    Pins,
    Styles,
}

/// Collected by a write's closure; empty means nothing to announce.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct Changes {
    pub issues: BTreeSet<String>,
    /// Issues whose detail alone changed (history, web links); their rows and the sidebar stay.
    pub details: BTreeSet<String>,
    pub local: BTreeSet<LocalPart>,
    /// The whole cache was replaced (Reset cache); every issue may have changed.
    pub cache_reset: bool,
}

impl Changes {
    pub fn issue_changed(&mut self, id: impl Into<String>) {
        self.issues.insert(id.into());
    }

    pub fn detail_changed(&mut self, id: impl Into<String>) {
        self.details.insert(id.into());
    }

    pub fn local_changed(&mut self, part: LocalPart) {
        self.local.insert(part);
    }

    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
            && self.details.is_empty()
            && self.local.is_empty()
            && !self.cache_reset
    }

    /// Adds what `other` changed, as if one write had recorded both.
    pub fn merge(&mut self, other: Changes) {
        self.issues.extend(other.issues);
        self.details.extend(other.details);
        self.local.extend(other.local);
        self.cache_reset |= other.cache_reset;
    }

    pub fn issues_event(&self) -> Option<IssuesChanged> {
        (self.cache_reset || !self.issues.is_empty()).then(|| IssuesChanged {
            ids: self.issues.iter().cloned().collect(),
            all: self.cache_reset,
        })
    }

    /// Issues whose detail changed but not as a whole; `issues_event` already covers the rest.
    pub fn details_event(&self) -> Option<IssuesChanged> {
        if self.cache_reset {
            return None;
        }
        let ids: Vec<String> = self.details.difference(&self.issues).cloned().collect();
        (!ids.is_empty()).then_some(IssuesChanged { ids, all: false })
    }

    pub fn local_event(&self) -> Option<LocalChanged> {
        (!self.local.is_empty()).then(|| LocalChanged {
            parts: self.local.iter().copied().collect(),
        })
    }
}

/// Payload of `issues_changed` and `issue_details_changed`.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct IssuesChanged {
    pub ids: Vec<String>,
    /// Every issue may have changed; `ids` is empty then.
    pub all: bool,
}

/// Payload of `local_changed`.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct LocalChanged {
    pub parts: Vec<LocalPart>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nothing_recorded_is_empty_and_has_no_events() {
        let changes = Changes::default();
        assert!(changes.is_empty());
        assert_eq!(changes.issues_event(), None);
        assert_eq!(changes.local_event(), None);
    }

    #[test]
    fn changed_issues_become_an_event_with_their_ids() {
        let mut changes = Changes::default();
        changes.issue_changed("20002");
        changes.issue_changed("20001");
        changes.issue_changed("20002");
        assert!(!changes.is_empty());
        assert_eq!(
            changes.issues_event(),
            Some(IssuesChanged {
                ids: vec!["20001".into(), "20002".into()],
                all: false
            })
        );
        assert_eq!(changes.local_event(), None);
    }

    #[test]
    fn a_cache_reset_is_an_event_for_all_issues() {
        let changes = Changes {
            cache_reset: true,
            ..Changes::default()
        };
        assert!(!changes.is_empty());
        assert_eq!(
            changes.issues_event(),
            Some(IssuesChanged {
                ids: vec![],
                all: true
            })
        );
    }

    #[test]
    fn a_detail_change_alone_is_not_empty_and_has_no_issues_event() {
        let mut changes = Changes::default();
        changes.detail_changed("20001");
        assert!(!changes.is_empty());
        assert_eq!(changes.issues_event(), None);
        assert_eq!(
            changes.details_event(),
            Some(IssuesChanged {
                ids: vec!["20001".into()],
                all: false
            })
        );
    }

    #[test]
    fn the_details_event_leaves_out_issues_that_changed_as_a_whole() {
        let mut changes = Changes::default();
        changes.detail_changed("20003");
        changes.detail_changed("20001");
        changes.detail_changed("20002");
        changes.issue_changed("20002");
        assert_eq!(
            changes.details_event(),
            Some(IssuesChanged {
                ids: vec!["20001".into(), "20003".into()],
                all: false
            })
        );
    }

    #[test]
    fn there_is_no_details_event_when_every_detail_is_covered_or_the_cache_was_reset() {
        let mut covered = Changes::default();
        covered.detail_changed("20001");
        covered.issue_changed("20001");
        assert_eq!(covered.details_event(), None);

        let mut reset = Changes {
            cache_reset: true,
            ..Changes::default()
        };
        reset.detail_changed("20001");
        assert_eq!(reset.details_event(), None);
        assert_eq!(Changes::default().details_event(), None);
    }

    #[test]
    fn merging_adds_everything_the_other_changes_recorded() {
        let mut first = Changes::default();
        first.issue_changed("20001");
        first.detail_changed("20002");
        first.local_changed(LocalPart::Pins);
        let mut second = Changes::default();
        second.issue_changed("20003");
        second.detail_changed("20002");
        second.local_changed(LocalPart::Views);
        second.cache_reset = true;

        first.merge(second);

        assert_eq!(
            first,
            Changes {
                issues: BTreeSet::from(["20001".to_string(), "20003".to_string()]),
                details: BTreeSet::from(["20002".to_string()]),
                local: BTreeSet::from([LocalPart::Views, LocalPart::Pins]),
                cache_reset: true,
            }
        );
    }

    #[test]
    fn merging_nothing_keeps_the_changes_as_they_were() {
        let mut changes = Changes::default();
        changes.issue_changed("20001");
        let before = changes.clone();
        changes.merge(Changes::default());
        assert_eq!(changes, before);
    }

    #[test]
    fn local_parts_are_listed_in_order_without_duplicates() {
        let mut changes = Changes::default();
        changes.local_changed(LocalPart::Styles);
        changes.local_changed(LocalPart::Lists);
        changes.local_changed(LocalPart::Spaces);
        changes.local_changed(LocalPart::Lists);
        assert!(!changes.is_empty());
        assert_eq!(changes.issues_event(), None);
        assert_eq!(
            changes.local_event(),
            Some(LocalChanged {
                parts: vec![LocalPart::Spaces, LocalPart::Lists, LocalPart::Styles]
            })
        );
    }

    #[test]
    fn event_payloads_serialize_in_camel_case() {
        assert_eq!(
            serde_json::to_value(IssuesChanged {
                ids: vec!["20001".into()],
                all: false
            })
            .unwrap(),
            json!({ "ids": ["20001"], "all": false })
        );
        assert_eq!(
            serde_json::to_value(LocalChanged {
                parts: vec![
                    LocalPart::Spaces,
                    LocalPart::Lists,
                    LocalPart::Views,
                    LocalPart::Pins,
                    LocalPart::Styles
                ]
            })
            .unwrap(),
            json!({ "parts": ["spaces", "lists", "views", "pins", "styles"] })
        );
    }
}
