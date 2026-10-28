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
#[derive(Default, Debug, PartialEq)]
pub struct Changes {
    pub issues: BTreeSet<String>,
    pub local: BTreeSet<LocalPart>,
    /// The whole cache was replaced (Reset cache); every issue may have changed.
    pub cache_reset: bool,
}

impl Changes {
    pub fn issue_changed(&mut self, id: impl Into<String>) {
        self.issues.insert(id.into());
    }

    pub fn local_changed(&mut self, part: LocalPart) {
        self.local.insert(part);
    }

    pub fn is_empty(&self) -> bool {
        self.issues.is_empty() && self.local.is_empty() && !self.cache_reset
    }

    pub fn issues_event(&self) -> Option<IssuesChanged> {
        (self.cache_reset || !self.issues.is_empty()).then(|| IssuesChanged {
            ids: self.issues.iter().cloned().collect(),
            all: self.cache_reset,
        })
    }

    pub fn local_event(&self) -> Option<LocalChanged> {
        (!self.local.is_empty()).then(|| LocalChanged {
            parts: self.local.iter().copied().collect(),
        })
    }
}

/// Payload of `issues_changed`.
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
