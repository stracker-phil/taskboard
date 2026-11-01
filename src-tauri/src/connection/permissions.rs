//! Whether Jira lets the account make Taskboard's changes in the tracked projects (Settings → Connection). Jira
//! answers for the user, not the token: a scoped token without `write:jira-work` still shows only when a change is
//! refused.

use serde::Serialize;
use std::collections::BTreeSet;
use ts_rs::TS;

use crate::jira::capability::Capability;

/// A change Jira doesn't allow in some tracked projects.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ChangeNotAllowed {
    pub capability: Capability,
    /// The projects' keys, in the sidebar's order.
    pub projects: Vec<String>,
}

/// Each change with the projects whose permissions (`granted`, per project key) lack what it needs; changes allowed
/// everywhere are left out.
pub fn not_allowed(granted: &[(String, BTreeSet<String>)]) -> Vec<ChangeNotAllowed> {
    Capability::ALL
        .iter()
        .filter_map(|&capability| {
            let needed = capability.jira_permission()?;
            let projects: Vec<String> = granted
                .iter()
                .filter(|(_, have)| !have.contains(needed))
                .map(|(key, _)| key.clone())
                .collect();
            (!projects.is_empty()).then_some(ChangeNotAllowed {
                capability,
                projects,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const JIRA_PERMISSIONS: [&str; 5] = [
        "ASSIGN_ISSUES",
        "TRANSITION_ISSUES",
        "EDIT_ISSUES",
        "ADD_COMMENTS",
        "LINK_ISSUES",
    ];

    fn project(key: &str, have: &[&str]) -> (String, BTreeSet<String>) {
        (
            key.to_string(),
            have.iter().map(|permission| permission.to_string()).collect(),
        )
    }

    fn without(denied: &[&str]) -> Vec<&'static str> {
        JIRA_PERMISSIONS.into_iter().filter(|p| !denied.contains(p)).collect()
    }

    #[test]
    fn nothing_is_reported_when_every_project_allows_every_change() {
        let granted = [
            project("NW", &JIRA_PERMISSIONS),
            project("OPS", &JIRA_PERMISSIONS),
        ];

        assert!(not_allowed(&granted).is_empty());
    }

    #[test]
    fn nothing_is_reported_without_projects() {
        assert!(not_allowed(&[]).is_empty());
    }

    #[test]
    fn a_change_lists_the_projects_lacking_its_permission_in_the_given_order() {
        let granted = [
            project("OPS", &without(&["EDIT_ISSUES"])),
            project("NW", &JIRA_PERMISSIONS),
            project("LUM", &without(&["EDIT_ISSUES"])),
        ];

        assert_eq!(
            not_allowed(&granted),
            [ChangeNotAllowed {
                capability: Capability::ChangePriority,
                projects: vec!["OPS".into(), "LUM".into()],
            }]
        );
    }

    #[test]
    fn changes_follow_the_capability_order_not_the_order_of_the_projects() {
        let granted = [
            project("NW", &without(&["LINK_ISSUES", "ASSIGN_ISSUES"])),
            project("OPS", &without(&["ADD_COMMENTS"])),
        ];

        assert_eq!(
            not_allowed(&granted),
            [
                ChangeNotAllowed {
                    capability: Capability::Assign,
                    projects: vec!["NW".into()],
                },
                ChangeNotAllowed {
                    capability: Capability::Comment,
                    projects: vec!["OPS".into()],
                },
                ChangeNotAllowed {
                    capability: Capability::AddWebLink,
                    projects: vec!["NW".into()],
                },
            ]
        );
    }

    #[test]
    fn a_project_with_no_permissions_lacks_every_change() {
        let granted = [project("NW", &JIRA_PERMISSIONS), project("OPS", &[])];

        let lacking: Vec<(Capability, Vec<String>)> = not_allowed(&granted)
            .into_iter()
            .map(|entry| (entry.capability, entry.projects))
            .collect();
        let ops = || vec!["OPS".to_string()];
        assert_eq!(
            lacking,
            [
                (Capability::Assign, ops()),
                (Capability::ChangeStatus, ops()),
                (Capability::ChangePriority, ops()),
                (Capability::Comment, ops()),
                (Capability::AddWebLink, ops()),
            ]
        );
    }

    #[test]
    fn a_not_allowed_change_serializes_with_camel_case_names() {
        let entry = ChangeNotAllowed {
            capability: Capability::ChangeStatus,
            projects: vec!["NW".into()],
        };

        assert_eq!(
            serde_json::to_value(&entry).unwrap(),
            serde_json::json!({ "capability": "changeStatus", "projects": ["NW"] })
        );
    }
}
