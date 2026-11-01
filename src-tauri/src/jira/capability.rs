//! What Taskboard does with Jira, one entry per kind of request. Every gateway endpoint names its capability
//! (`Endpoint::capability`), so this list can't drift from what the app sends. Settings → Connection lists the reads
//! and lets the owner switch each kind of change on or off (`Behaviour::disabled_changes`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Capability {
    Account,
    Projects,
    Issues,
    IssueDetails,
    CheckQueries,
    Assign,
    ChangeStatus,
    ChangePriority,
    ChangeTitle,
    Comment,
    AddWebLink,
}

impl Capability {
    pub const ALL: [Self; 11] = [
        Self::Account,
        Self::Projects,
        Self::Issues,
        Self::IssueDetails,
        Self::CheckQueries,
        Self::Assign,
        Self::ChangeStatus,
        Self::ChangePriority,
        Self::ChangeTitle,
        Self::Comment,
        Self::AddWebLink,
    ];

    /// Changes Jira; the rest only read.
    pub fn is_change(self) -> bool {
        matches!(
            self,
            Self::Assign
                | Self::ChangeStatus
                | Self::ChangePriority
                | Self::ChangeTitle
                | Self::Comment
                | Self::AddWebLink
        )
    }

    /// The scope a scoped API token needs for it, as Jira names it (`docs/scopes.md`).
    pub fn scope(self) -> &'static str {
        match self {
            Self::Account => "read:jira-user",
            Self::Projects | Self::Issues | Self::IssueDetails | Self::CheckQueries => {
                "read:jira-work"
            }
            Self::Assign
            | Self::ChangeStatus
            | Self::ChangePriority
            | Self::ChangeTitle
            | Self::Comment
            | Self::AddWebLink => "write:jira-work",
        }
    }

    /// The Jira project permission a change needs in the issue's project; `None` for a read.
    pub fn jira_permission(self) -> Option<&'static str> {
        match self {
            Self::Assign => Some("ASSIGN_ISSUES"),
            Self::ChangeStatus => Some("TRANSITION_ISSUES"),
            Self::ChangePriority | Self::ChangeTitle => Some("EDIT_ISSUES"),
            Self::Comment => Some("ADD_COMMENTS"),
            Self::AddWebLink => Some("LINK_ISSUES"),
            Self::Account
            | Self::Projects
            | Self::Issues
            | Self::IssueDetails
            | Self::CheckQueries => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Account => "Your account",
            Self::Projects => "List projects",
            Self::Issues => "Read issues",
            Self::IssueDetails => "Read an issue's details",
            Self::CheckQueries => "Check filter queries",
            Self::Assign => "Change the assignee",
            Self::ChangeStatus => "Change the status",
            Self::ChangePriority => "Change the priority",
            Self::ChangeTitle => "Change the title",
            Self::Comment => "Add comments",
            Self::AddWebLink => "Add web links",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Self::Account => "Your name, account and time zone, to know which issues are yours",
            Self::Projects => "Projects, their statuses, fields and releases",
            Self::Issues => "The issues in the sync scope, with their fields",
            Self::IssueDetails => {
                "Comments, history, web links, attachments and the statuses and priorities an issue can take"
            }
            Self::CheckQueries => "Whether the sync scope's query is valid",
            Self::Assign => "Assign issues to you or a teammate, or unassign them",
            Self::ChangeStatus => "Move issues along their workflow",
            Self::ChangePriority => "Set an issue's priority from its menu, or by dragging it",
            Self::ChangeTitle => "Rename an issue in the Issue view or the Preview",
            Self::Comment => "Comment on issues from the Issue view",
            Self::AddWebLink => "Add links such as a pull request to issues",
        }
    }
}

/// A capability as Settings → Connection shows it.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRow {
    pub capability: Capability,
    pub label: String,
    pub detail: String,
    /// The token scope it needs, as Jira names it.
    pub scope: String,
    pub change: bool,
    /// A change: allowed now (changes to Jira on and this one not switched off). A read: always.
    pub enabled: bool,
}

/// Every capability, reads first, as allowed by `allow_changes` and `disabled`.
pub fn capability_rows(allow_changes: bool, disabled: &[Capability]) -> Vec<CapabilityRow> {
    Capability::ALL
        .iter()
        .map(|&capability| CapabilityRow {
            capability,
            label: capability.label().into(),
            detail: capability.detail().into(),
            scope: capability.scope().into(),
            change: capability.is_change(),
            enabled: !capability.is_change() || (allow_changes && !disabled.contains(&capability)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const CHANGES: [Capability; 6] = [
        Capability::Assign,
        Capability::ChangeStatus,
        Capability::ChangePriority,
        Capability::ChangeTitle,
        Capability::Comment,
        Capability::AddWebLink,
    ];

    #[test]
    fn the_list_holds_every_capability_once() {
        let unique: HashSet<Capability> = Capability::ALL.into_iter().collect();

        assert_eq!(unique.len(), Capability::ALL.len());
        assert_eq!(unique.len(), 11);
    }

    #[test]
    fn the_list_has_the_reads_first_and_then_the_changes() {
        let flags: Vec<bool> = Capability::ALL.iter().map(|c| c.is_change()).collect();

        assert_eq!(
            flags,
            [
                false, false, false, false, false, true, true, true, true, true, true
            ]
        );
    }

    #[test]
    fn exactly_the_six_kinds_of_write_are_changes() {
        let changes: Vec<Capability> = Capability::ALL
            .into_iter()
            .filter(|c| c.is_change())
            .collect();

        assert_eq!(changes, CHANGES);
    }

    #[test]
    fn every_capability_has_a_label_and_a_detail() {
        for capability in Capability::ALL {
            assert!(!capability.label().is_empty(), "{capability:?}");
            assert!(!capability.detail().is_empty(), "{capability:?}");
        }
    }

    #[test]
    fn reads_are_always_enabled() {
        for (allow, disabled) in [
            (false, vec![]),
            (true, vec![]),
            (false, Capability::ALL.to_vec()),
            (true, Capability::ALL.to_vec()),
        ] {
            for row in capability_rows(allow, &disabled)
                .iter()
                .filter(|row| !row.change)
            {
                assert!(row.enabled, "{:?} allow={allow}", row.capability);
            }
        }
    }

    #[test]
    fn changes_are_all_off_while_changes_to_jira_are_off() {
        for row in capability_rows(false, &[]).iter().filter(|row| row.change) {
            assert!(!row.enabled, "{:?}", row.capability);
        }
    }

    #[test]
    fn changes_are_all_on_when_allowed_and_none_is_disabled() {
        for row in capability_rows(true, &[]).iter().filter(|row| row.change) {
            assert!(row.enabled, "{:?}", row.capability);
        }
    }

    #[test]
    fn a_disabled_change_is_off_while_the_others_stay_on() {
        let rows = capability_rows(true, &[Capability::Comment, Capability::ChangePriority]);

        let enabled: Vec<(Capability, bool)> = rows
            .iter()
            .filter(|row| row.change)
            .map(|row| (row.capability, row.enabled))
            .collect();
        assert_eq!(
            enabled,
            [
                (Capability::Assign, true),
                (Capability::ChangeStatus, true),
                (Capability::ChangePriority, false),
                (Capability::ChangeTitle, true),
                (Capability::Comment, false),
                (Capability::AddWebLink, true),
            ]
        );
    }

    #[test]
    fn rows_follow_the_list_and_carry_the_label_detail_and_change_flag() {
        let rows = capability_rows(true, &[]);

        assert_eq!(rows.len(), Capability::ALL.len());
        for (row, capability) in rows.iter().zip(Capability::ALL) {
            assert_eq!(row.capability, capability);
            assert_eq!(row.label, capability.label(), "{capability:?}");
            assert_eq!(row.detail, capability.detail(), "{capability:?}");
            assert_eq!(row.change, capability.is_change(), "{capability:?}");
        }
    }

    #[test]
    fn every_change_needs_the_write_scope_and_no_read_does() {
        for capability in Capability::ALL {
            assert_eq!(
                capability.scope() == "write:jira-work",
                capability.is_change(),
                "{capability:?}"
            );
        }
    }

    #[test]
    fn each_capability_names_the_scope_jira_requires() {
        for (capability, scope) in [
            (Capability::Account, "read:jira-user"),
            (Capability::Projects, "read:jira-work"),
            (Capability::Issues, "read:jira-work"),
            (Capability::IssueDetails, "read:jira-work"),
            (Capability::CheckQueries, "read:jira-work"),
            (Capability::Assign, "write:jira-work"),
            (Capability::ChangeStatus, "write:jira-work"),
            (Capability::ChangePriority, "write:jira-work"),
            (Capability::ChangeTitle, "write:jira-work"),
            (Capability::Comment, "write:jira-work"),
            (Capability::AddWebLink, "write:jira-work"),
        ] {
            assert_eq!(capability.scope(), scope, "{capability:?}");
        }
    }

    #[test]
    fn each_change_names_the_jira_permission_it_needs() {
        for (capability, permission) in [
            (Capability::Assign, "ASSIGN_ISSUES"),
            (Capability::ChangeStatus, "TRANSITION_ISSUES"),
            (Capability::ChangePriority, "EDIT_ISSUES"),
            (Capability::ChangeTitle, "EDIT_ISSUES"),
            (Capability::Comment, "ADD_COMMENTS"),
            (Capability::AddWebLink, "LINK_ISSUES"),
        ] {
            assert_eq!(
                capability.jira_permission(),
                Some(permission),
                "{capability:?}"
            );
        }
    }

    #[test]
    fn every_change_needs_a_jira_permission_and_no_read_does() {
        for capability in Capability::ALL {
            assert_eq!(
                capability.jira_permission().is_some(),
                capability.is_change(),
                "{capability:?}"
            );
        }
    }

    #[test]
    fn rows_carry_the_scope_of_their_capability() {
        for row in capability_rows(true, &[]) {
            assert_eq!(row.scope, row.capability.scope(), "{:?}", row.capability);
        }
    }

    #[test]
    fn changing_the_title_is_a_labeled_change_after_the_priority() {
        let position = |c: Capability| Capability::ALL.iter().position(|&x| x == c).unwrap();

        assert_eq!(Capability::ChangeTitle.label(), "Change the title");
        assert!(Capability::ChangeTitle.is_change());
        assert_eq!(
            position(Capability::ChangeTitle),
            position(Capability::ChangePriority) + 1
        );
        assert_eq!(
            serde_json::to_value(Capability::ChangeTitle).unwrap(),
            serde_json::json!("changeTitle")
        );
    }

    #[test]
    fn a_row_serializes_with_camel_case_names() {
        let row = capability_rows(true, &[])
            .into_iter()
            .find(|row| row.capability == Capability::AddWebLink)
            .unwrap();

        assert_eq!(
            serde_json::to_value(&row).unwrap(),
            serde_json::json!({
                "capability": "addWebLink",
                "label": "Add web links",
                "detail": "Add links such as a pull request to issues",
                "scope": "write:jira-work",
                "change": true,
                "enabled": true,
            })
        );
    }
}
