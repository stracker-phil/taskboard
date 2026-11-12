//! What turns an issue unread (In app) and what macOS announces (Settings → Notifications, `docs/notifications.md`).

use std::fs;
use std::path::Path;

use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::sync::tidy_types;

/// A kind of change, as the Notifications page lists it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum NotifyEvent {
    MentionsMe,
    AssignsMe,
    UnassignsMe,
    /// The connected account was added to a custom people field (Developer, Reviewer, …).
    AddsMeToPeopleField,
    NewComment,
    MovedToDone,
    /// Back to a To Do status from another category.
    Reopened,
    /// Any other status change, and an issue updated while the cache didn't hold it.
    OtherChange,
    DueOrPriority,
    Created,
}

impl NotifyEvent {
    pub const ALL: [NotifyEvent; 10] = [
        NotifyEvent::MentionsMe,
        NotifyEvent::AssignsMe,
        NotifyEvent::UnassignsMe,
        NotifyEvent::AddsMeToPeopleField,
        NotifyEvent::NewComment,
        NotifyEvent::MovedToDone,
        NotifyEvent::Reopened,
        NotifyEvent::OtherChange,
        NotifyEvent::DueOrPriority,
        NotifyEvent::Created,
    ];

    /// Always counts when ticked: skips the scope and excluded-type filters.
    pub fn is_about_me(self) -> bool {
        matches!(
            self,
            NotifyEvent::MentionsMe
                | NotifyEvent::AssignsMe
                | NotifyEvent::UnassignsMe
                | NotifyEvent::AddsMeToPeopleField
        )
    }
}

/// Which issues the other events are judged on.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum NotifyScope {
    #[default]
    AllIssues,
    /// Only issues the owner is involved in (`Involvement`).
    Involved,
}

/// What makes the owner involved in an issue.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Involvement {
    pub assigned: bool,
    pub reported: bool,
    /// Named in a fetched custom people field.
    pub people_fields: bool,
}

impl Default for Involvement {
    fn default() -> Self {
        Self {
            assigned: true,
            reported: true,
            people_fields: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(default, rename_all = "camelCase")]
pub struct Notifications {
    pub scope: NotifyScope,
    pub involvement: Involvement,
    /// Issue types (by name) whose On-these-issues events count nowhere.
    pub excluded_types: Vec<String>,
    /// The events that make an issue unread.
    pub in_app: Vec<NotifyEvent>,
    /// The events a macOS notification announces after a change sync.
    pub mac: Vec<NotifyEvent>,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            scope: NotifyScope::default(),
            involvement: Involvement::default(),
            excluded_types: Vec::new(),
            in_app: NotifyEvent::ALL.to_vec(),
            mac: vec![
                NotifyEvent::MentionsMe,
                NotifyEvent::AssignsMe,
                NotifyEvent::AddsMeToPeopleField,
            ],
        }
    }
}

/// The two places a change can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    InApp,
    Mac,
}

impl Notifications {
    pub fn events(&self, channel: Channel) -> &[NotifyEvent] {
        match channel {
            Channel::InApp => &self.in_app,
            Channel::Mac => &self.mac,
        }
    }

    /// Event lists in the page's order without repeats, the excluded types tidied.
    pub fn tidied(self) -> Self {
        let ordered = |events: Vec<NotifyEvent>| -> Vec<NotifyEvent> {
            NotifyEvent::ALL
                .into_iter()
                .filter(|e| events.contains(e))
                .collect()
        };
        Self {
            excluded_types: tidy_types(self.excluded_types),
            in_app: ordered(self.in_app),
            mac: ordered(self.mac),
            ..self
        }
    }

    /// The Behavior page's old choice, carried over once. The filters are shared by both channels, so in app
    /// stays as it was (all issues, every event) and only the macOS events follow the old choice.
    pub fn from_notify_about(about: NotifyAbout) -> Self {
        match about {
            NotifyAbout::Nothing => Self {
                mac: Vec::new(),
                ..Self::default()
            },
            NotifyAbout::MentionsAndAssignments | NotifyAbout::MyIssues => Self::default(),
        }
    }
}

/// What macOS notifications announced before Settings → Notifications (`behaviour.notifyAbout`).
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotifyAbout {
    Nothing,
    MentionsAndAssignments,
    MyIssues,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OldFile {
    #[serde(default)]
    behaviour: OldBehaviour,
    notifications: Option<IgnoredAny>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct OldBehaviour {
    notify_about: Option<NotifyAbout>,
}

/// The old choice in a settings file without `notifications`; it is read once and never written back.
pub fn old_notify_about(path: &Path) -> Option<NotifyAbout> {
    let file: OldFile = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    match file.notifications {
        Some(_) => None,
        None => file.behaviour.notify_about,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::settings::Settings;
    use crate::test_support::temp_config_dir;
    use serde_json::json;

    fn default_mac() -> Vec<NotifyEvent> {
        vec![
            NotifyEvent::MentionsMe,
            NotifyEvent::AssignsMe,
            NotifyEvent::AddsMeToPeopleField,
        ]
    }

    #[test]
    fn the_defaults_show_every_event_in_app_and_announce_what_is_about_me() {
        let defaults = Notifications::default();

        assert_eq!(defaults.scope, NotifyScope::AllIssues);
        assert_eq!(defaults.involvement, Involvement::default());
        assert!(defaults.excluded_types.is_empty());
        assert_eq!(defaults.in_app, NotifyEvent::ALL.to_vec());
        assert_eq!(defaults.mac, default_mac());
    }

    #[test]
    fn each_channel_reads_its_own_event_list() {
        let settings = Notifications {
            in_app: vec![NotifyEvent::NewComment],
            mac: vec![NotifyEvent::Created],
            ..Notifications::default()
        };

        assert_eq!(
            settings.events(Channel::InApp),
            [NotifyEvent::NewComment].as_slice()
        );
        assert_eq!(
            settings.events(Channel::Mac),
            [NotifyEvent::Created].as_slice()
        );
    }

    #[test]
    fn exactly_the_four_events_about_the_owner_are_about_me() {
        let about_me: Vec<NotifyEvent> = NotifyEvent::ALL
            .into_iter()
            .filter(|e| e.is_about_me())
            .collect();

        assert_eq!(
            about_me,
            [
                NotifyEvent::MentionsMe,
                NotifyEvent::AssignsMe,
                NotifyEvent::UnassignsMe,
                NotifyEvent::AddsMeToPeopleField,
            ]
        );
    }

    #[test]
    fn nothing_turns_the_mac_events_off_and_leaves_in_app_as_it_was() {
        assert_eq!(
            Notifications::from_notify_about(NotifyAbout::Nothing),
            Notifications {
                mac: Vec::new(),
                ..Notifications::default()
            }
        );
    }

    #[test]
    fn the_other_old_choices_give_the_default_events_on_all_issues() {
        for about in [NotifyAbout::MentionsAndAssignments, NotifyAbout::MyIssues] {
            let settings = Notifications::from_notify_about(about);

            assert_eq!(settings.mac, default_mac(), "{about:?}");
            assert_eq!(settings.scope, NotifyScope::AllIssues, "{about:?}");
            assert_eq!(settings.in_app, NotifyEvent::ALL.to_vec(), "{about:?}");
        }
    }

    #[test]
    fn tidying_orders_events_like_the_page_and_drops_repeats() {
        let tidied = Notifications {
            in_app: vec![
                NotifyEvent::Created,
                NotifyEvent::MentionsMe,
                NotifyEvent::Created,
                NotifyEvent::NewComment,
            ],
            mac: vec![
                NotifyEvent::DueOrPriority,
                NotifyEvent::AssignsMe,
                NotifyEvent::AssignsMe,
            ],
            ..Notifications::default()
        }
        .tidied();

        assert_eq!(
            tidied.in_app,
            [
                NotifyEvent::MentionsMe,
                NotifyEvent::NewComment,
                NotifyEvent::Created
            ]
        );
        assert_eq!(
            tidied.mac,
            [NotifyEvent::AssignsMe, NotifyEvent::DueOrPriority]
        );
    }

    #[test]
    fn tidying_trims_sorts_and_dedupes_the_excluded_types() {
        let tidied = Notifications {
            excluded_types: vec![
                " Task ".into(),
                "Bug".into(),
                "".into(),
                "Bug".into(),
                "  ".into(),
            ],
            ..Notifications::default()
        }
        .tidied();

        assert_eq!(tidied.excluded_types, ["Bug", "Task"]);
    }

    #[test]
    fn tidying_keeps_the_scope_and_involvement() {
        let settings = Notifications {
            scope: NotifyScope::Involved,
            involvement: Involvement {
                assigned: false,
                reported: true,
                people_fields: false,
            },
            ..Notifications::default()
        };

        assert_eq!(settings.clone().tidied(), settings);
    }

    #[test]
    fn a_settings_file_without_notifications_reads_the_defaults() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.notifications, Notifications::default());
    }

    #[test]
    fn a_partial_notifications_object_keeps_the_rest_default() {
        let settings: Settings =
            serde_json::from_str(r#"{"notifications":{"scope":"involved"}}"#).unwrap();

        assert_eq!(
            settings.notifications,
            Notifications {
                scope: NotifyScope::Involved,
                ..Notifications::default()
            }
        );
    }

    #[test]
    fn notifications_serialize_with_camel_case_names() {
        assert_eq!(
            serde_json::to_value(Notifications::default()).unwrap(),
            json!({
                "scope": "allIssues",
                "involvement": {"assigned": true, "reported": true, "peopleFields": true},
                "excludedTypes": [],
                "inApp": [
                    "mentionsMe", "assignsMe", "unassignsMe", "addsMeToPeopleField", "newComment",
                    "movedToDone", "reopened", "otherChange", "dueOrPriority", "created"
                ],
                "mac": ["mentionsMe", "assignsMe", "addsMeToPeopleField"],
            })
        );
    }

    #[test]
    fn the_old_choice_is_read_only_from_a_file_without_notifications() {
        let cases = [
            (
                r#"{"behaviour":{"notifyAbout":"nothing"}}"#,
                Some(NotifyAbout::Nothing),
            ),
            (
                r#"{"behaviour":{"notifyAbout":"myIssues"}}"#,
                Some(NotifyAbout::MyIssues),
            ),
            (
                r#"{"behaviour":{"notifyAbout":"nothing"},"notifications":{}}"#,
                None,
            ),
            (r#"{"behaviour":{}}"#, None),
            (r#"{"behaviour":{"notifyAbout":"sometimes"}}"#, None),
            ("{ not json", None),
        ];
        for (file, expected) in cases {
            let dir = temp_config_dir();
            let path = dir.path().join("settings.json");
            fs::write(&path, file).unwrap();

            assert_eq!(old_notify_about(&path), expected, "{file}");
        }
    }

    #[test]
    fn without_a_file_there_is_no_old_choice() {
        let dir = temp_config_dir();

        assert_eq!(old_notify_about(&dir.path().join("settings.json")), None);
    }
}
