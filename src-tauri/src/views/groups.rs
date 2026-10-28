//! Splits sorted rows into a view's groups, each in its fixed order (docs/data.md, Group keys).

use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;
use ts_rs::TS;

use super::model::GroupBy;
use super::rows::{Person, Row};
use crate::status_style::Style;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub key: String,
    pub label: String,
    pub mark: Option<GroupMark>,
    pub count: usize,
    pub collapsed_by_default: bool,
    pub row_ids: Vec<String>,
}

/// What the group header draws before its label.
#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GroupMark {
    /// The label shows as a status pill.
    Status {
        style: Style,
    },
    Space {
        letter: String,
        color: Option<String>,
    },
    Person {
        person: Person,
    },
    Epic {
        color: u8,
    },
    Priority {
        level: u8,
    },
}

/// What a row is grouped by, besides what `Row` shows.
pub struct Facts {
    pub status_id: String,
    pub project_id: String,
    pub assignee_id: Option<String>,
    pub release_id: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub updated_at: String,
    pub created_at: String,
}

pub struct SpaceInfo {
    pub position: usize,
    pub name: String,
    pub letter: String,
    pub color: Option<String>,
    pub client: Option<String>,
}

pub struct Version {
    pub name: String,
    pub release_date: Option<String>,
}

/// Reference data for grouping; `me` is the connected account, which comes first under Assignee.
pub struct Lookup<'a> {
    pub me: Option<&'a str>,
    pub today: NaiveDate,
    pub spaces: &'a HashMap<String, SpaceInfo>,
    pub versions: &'a HashMap<String, Version>,
}

/// Groups sort by `(rank, text)`; rows keep their order inside a group.
type Order = (i64, String);

/// `rows` already sorted; groups come out in their order, empty ones left out.
pub fn group(rows: &[(Row, Facts)], by: GroupBy, lookup: &Lookup) -> Vec<Group> {
    let mut groups: BTreeMap<Order, Group> = BTreeMap::new();
    for (row, facts) in rows {
        let (order, empty) = slot_of(row, facts, by, lookup);
        let group = groups.entry(order).or_insert(empty);
        group.row_ids.push(row.id.clone());
        group.count += 1;
    }
    groups.into_values().collect()
}

/// The row's group, still empty, with its place among the groups.
fn slot_of(row: &Row, facts: &Facts, by: GroupBy, lookup: &Lookup) -> (Order, Group) {
    let slot = |key: &str, label: &str, mark: Option<GroupMark>, order: Order| {
        let group = Group {
            collapsed_by_default: matches!(mark, Some(GroupMark::Status { style: Style::Done })),
            key: key.to_string(),
            label: label.to_string(),
            mark,
            count: 0,
            row_ids: Vec::new(),
        };
        (order, group)
    };
    let last = |key: &str, label: &str| slot(key, label, None, (i64::MAX, String::new()));
    match by {
        GroupBy::Status => slot(
            &facts.status_id,
            &row.status_name,
            Some(GroupMark::Status { style: row.style }),
            (row.style as i64, row.status_name.to_lowercase()),
        ),
        GroupBy::Project => match lookup.spaces.get(&facts.project_id) {
            Some(space) => slot(
                &facts.project_id,
                &space.name,
                Some(GroupMark::Space {
                    letter: space.letter.clone(),
                    color: space.color.clone(),
                }),
                (space.position as i64, String::new()),
            ),
            None => last(&facts.project_id, "Other"),
        },
        GroupBy::Client => match lookup
            .spaces
            .get(&facts.project_id)
            .and_then(|space| space.client.as_deref())
        {
            Some(client) => slot(client, client, None, (0, client.to_lowercase())),
            None => last("", "Internal"),
        },
        GroupBy::Assignee => match (&facts.assignee_id, row.assignees.first()) {
            (Some(id), Some(assignee)) => slot(
                id,
                &assignee.name,
                Some(GroupMark::Person {
                    person: assignee.clone(),
                }),
                (
                    i64::from(lookup.me != Some(id)),
                    assignee.name.to_lowercase(),
                ),
            ),
            _ => last("", "Unassigned"),
        },
        GroupBy::Epic => match &row.epic {
            Some(epic) => slot(
                &epic.id,
                &epic.name,
                Some(GroupMark::Epic { color: epic.color }),
                (0, epic.name.to_lowercase()),
            ),
            None => last("", "No epic"),
        },
        GroupBy::Release => match facts
            .release_id
            .as_ref()
            .and_then(|id| lookup.versions.get(id).map(|version| (id, version)))
        {
            // Dated releases by date, then undated ones by name.
            Some((id, version)) => match &version.release_date {
                Some(date) => slot(
                    id,
                    &version.name,
                    None,
                    (0, format!("{date} {}", version.name)),
                ),
                None => slot(id, &version.name, None, (1, version.name.to_lowercase())),
            },
            None => last("", "No release"),
        },
        GroupBy::Priority => match &row.priority {
            Some(priority) => slot(
                &priority.name,
                &priority.name,
                Some(GroupMark::Priority {
                    level: priority.level,
                }),
                (i64::from(priority.level), priority.name.to_lowercase()),
            ),
            None => last("", "No priority"),
        },
        GroupBy::Due => {
            let bucket = due_bucket(facts.due_date, lookup.today);
            slot(
                bucket.key(),
                bucket.label(),
                None,
                (bucket.rank(), String::new()),
            )
        }
        GroupBy::None => slot("all", "", None, (0, String::new())),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DueBucket {
    Overdue,
    Today,
    ThisWeek,
    Later,
    None,
}

impl DueBucket {
    fn rank(self) -> i64 {
        match self {
            Self::Overdue => 0,
            Self::Today => 1,
            Self::ThisWeek => 2,
            Self::Later => 3,
            Self::None => 4,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Overdue => "overdue",
            Self::Today => "today",
            Self::ThisWeek => "thisWeek",
            Self::Later => "later",
            Self::None => "none",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Overdue => "Overdue",
            Self::Today => "Today",
            Self::ThisWeek => "This week",
            Self::Later => "Later",
            Self::None => "No due date",
        }
    }
}

/// Weeks end on Sunday.
fn due_bucket(due: Option<NaiveDate>, today: NaiveDate) -> DueBucket {
    let Some(due) = due else {
        return DueBucket::None;
    };
    let sunday = today + Duration::days(6 - i64::from(today.weekday().num_days_from_monday()));
    if due < today {
        DueBucket::Overdue
    } else if due == today {
        DueBucket::Today
    } else if due <= sunday {
        DueBucket::ThisWeek
    } else {
        DueBucket::Later
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::rows::{EpicChip, person, priority};

    fn date(text: &str) -> NaiveDate {
        text.parse().unwrap()
    }

    fn row(id: &str) -> Row {
        Row {
            id: id.to_string(),
            key: format!("DEMO-{id}"),
            summary: format!("Task {id}"),
            style: Style::Progress,
            status_name: "In Progress".to_string(),
            comment_count: 0,
            changed: false,
            change_note: None,
            assignees: Vec::new(),
            epic: None,
            release: None,
            priority: None,
            labels: Vec::new(),
            sprint: None,
            due: None,
            custom: BTreeMap::new(),
        }
    }

    fn facts() -> Facts {
        Facts {
            status_id: "1".to_string(),
            project_id: "p1".to_string(),
            assignee_id: None,
            release_id: None,
            due_date: None,
            updated_at: String::new(),
            created_at: String::new(),
        }
    }

    fn with_status(id: &str, style: Style, name: &str) -> (Row, Facts) {
        let mut row = row(id);
        row.style = style;
        row.status_name = name.to_string();
        let mut facts = facts();
        facts.status_id = name.to_lowercase();
        (row, facts)
    }

    fn run(rows: &[(Row, Facts)], by: GroupBy) -> Vec<Group> {
        run_with(rows, by, &HashMap::new(), &HashMap::new())
    }

    fn run_with(
        rows: &[(Row, Facts)],
        by: GroupBy,
        spaces: &HashMap<String, SpaceInfo>,
        versions: &HashMap<String, Version>,
    ) -> Vec<Group> {
        let lookup = Lookup {
            me: Some("acc-me"),
            today: date("2026-10-28"),
            spaces,
            versions,
        };
        group(rows, by, &lookup)
    }

    fn labels(groups: &[Group]) -> Vec<&str> {
        groups.iter().map(|group| group.label.as_str()).collect()
    }

    #[test]
    fn due_dates_fall_into_buckets_with_weeks_ending_on_sunday() {
        let wednesday = date("2026-10-28");
        let sunday = date("2026-11-01");
        for (today, due, expected) in [
            (wednesday, None, DueBucket::None),
            (wednesday, Some("2026-10-27"), DueBucket::Overdue),
            (wednesday, Some("2026-10-28"), DueBucket::Today),
            (wednesday, Some("2026-11-01"), DueBucket::ThisWeek),
            (wednesday, Some("2026-11-02"), DueBucket::Later),
            (sunday, Some("2026-11-02"), DueBucket::Later),
        ] {
            assert_eq!(
                due_bucket(due.map(date), today),
                expected,
                "{today} {due:?}"
            );
        }
    }

    #[test]
    fn status_groups_follow_the_style_rank_and_only_done_starts_collapsed() {
        let rows = [
            with_status("1", Style::Done, "Done"),
            with_status("2", Style::Todo, "To Do"),
            with_status("3", Style::Progress, "In Progress"),
            with_status("4", Style::Blocked, "Blocked"),
            with_status("5", Style::Review, "In Review"),
            with_status("6", Style::Progress, "In Progress"),
        ];
        let groups = run(&rows, GroupBy::Status);

        assert_eq!(
            labels(&groups),
            ["In Progress", "In Review", "Blocked", "To Do", "Done"]
        );
        assert_eq!(
            groups
                .iter()
                .map(|g| g.collapsed_by_default)
                .collect::<Vec<_>>(),
            [false, false, false, false, true]
        );
        assert_eq!(groups[0].row_ids, ["3", "6"]);
        assert_eq!(groups[0].count, 2);
        assert_eq!(
            groups[0].mark,
            Some(GroupMark::Status {
                style: Style::Progress
            })
        );
        assert_eq!(groups[0].key, "in progress");
    }

    #[test]
    fn grouping_by_nothing_gives_one_group_with_every_row() {
        let rows = [
            with_status("2", Style::Done, "Done"),
            with_status("1", Style::Todo, "To Do"),
        ];
        let groups = run(&rows, GroupBy::None);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].key, "all");
        assert_eq!(groups[0].row_ids, ["2", "1"]);
        assert_eq!(groups[0].count, 2);
        assert!(!groups[0].collapsed_by_default);
    }

    #[test]
    fn no_rows_give_no_groups() {
        assert!(run(&[], GroupBy::Status).is_empty());
    }

    #[test]
    fn assignee_groups_put_me_first_and_unassigned_last() {
        let assigned = |id: &str, account: &str, name: &str| {
            let mut row = row(id);
            row.assignees = vec![person(account, name, Some("acc-me"))];
            let mut facts = facts();
            facts.assignee_id = Some(account.to_string());
            (row, facts)
        };
        let rows = [
            (row("1"), facts()),
            assigned("2", "acc-ana", "Ana Lopez"),
            assigned("3", "acc-me", "Zed Morgan"),
        ];
        let groups = run(&rows, GroupBy::Assignee);

        assert_eq!(labels(&groups), ["Zed Morgan", "Ana Lopez", "Unassigned"]);
        assert_eq!(groups[0].key, "acc-me");
        assert_eq!(groups[2].mark, None);
    }

    #[test]
    fn epic_groups_are_alphabetical_with_no_epic_last() {
        let epic = |id: &str, name: &str| {
            let mut row = row(id);
            row.epic = Some(EpicChip {
                id: format!("e-{name}"),
                name: name.to_string(),
                color: 2,
            });
            (row, facts())
        };
        let rows = [
            (row("1"), facts()),
            epic("2", "Onboarding"),
            epic("3", "Billing"),
        ];
        let groups = run(&rows, GroupBy::Epic);

        assert_eq!(labels(&groups), ["Billing", "Onboarding", "No epic"]);
        assert_eq!(groups[0].mark, Some(GroupMark::Epic { color: 2 }));
    }

    #[test]
    fn due_groups_run_from_overdue_to_no_due_date() {
        let due = |id: &str, date_text: Option<&str>| {
            let mut facts = facts();
            facts.due_date = date_text.map(date);
            (row(id), facts)
        };
        let rows = [
            due("1", None),
            due("2", Some("2026-11-02")),
            due("3", Some("2026-11-01")),
            due("4", Some("2026-10-28")),
            due("5", Some("2026-10-01")),
        ];
        let groups = run(&rows, GroupBy::Due);

        assert_eq!(
            labels(&groups),
            ["Overdue", "Today", "This week", "Later", "No due date"]
        );
        assert_eq!(
            groups.iter().map(|g| g.key.as_str()).collect::<Vec<_>>(),
            ["overdue", "today", "thisWeek", "later", "none"]
        );
    }

    fn space(position: usize, name: &str, client: Option<&str>) -> SpaceInfo {
        SpaceInfo {
            position,
            name: name.to_string(),
            letter: name[..1].to_string(),
            color: None,
            client: client.map(str::to_string),
        }
    }

    fn in_project(id: &str, project: &str) -> (Row, Facts) {
        let mut facts = facts();
        facts.project_id = project.to_string();
        (row(id), facts)
    }

    #[test]
    fn project_groups_follow_the_space_position_and_use_the_space_name() {
        let spaces = HashMap::from([
            ("p1".to_string(), space(1, "Bravo Web", None)),
            ("p2".to_string(), space(0, "Alpha App", None)),
        ]);
        let rows = [in_project("1", "p1"), in_project("2", "p2")];
        let groups = run_with(&rows, GroupBy::Project, &spaces, &HashMap::new());

        assert_eq!(labels(&groups), ["Alpha App", "Bravo Web"]);
        assert_eq!(
            groups[0].mark,
            Some(GroupMark::Space {
                letter: "A".to_string(),
                color: None
            })
        );
    }

    #[test]
    fn client_groups_are_alphabetical_with_internal_last() {
        let spaces = HashMap::from([
            (
                "p1".to_string(),
                space(0, "Alpha App", Some("Zeta Example")),
            ),
            (
                "p2".to_string(),
                space(1, "Bravo Web", Some("Acme Example")),
            ),
            ("p3".to_string(), space(2, "Charlie Ops", None)),
        ]);
        let rows = [
            in_project("1", "p1"),
            in_project("2", "p3"),
            in_project("3", "p2"),
        ];
        let groups = run_with(&rows, GroupBy::Client, &spaces, &HashMap::new());

        assert_eq!(
            labels(&groups),
            ["Acme Example", "Zeta Example", "Internal"]
        );
    }

    #[test]
    fn release_groups_follow_the_release_date_with_no_release_last() {
        let version = |name: &str, release_date: Option<&str>| Version {
            name: name.to_string(),
            release_date: release_date.map(str::to_string),
        };
        let versions = HashMap::from([
            ("v1".to_string(), version("1.2", Some("2026-12-01"))),
            ("v2".to_string(), version("1.1", Some("2026-11-01"))),
            ("v3".to_string(), version("2.0", None)),
        ]);
        let released = |id: &str, release: Option<&str>| {
            let mut facts = facts();
            facts.release_id = release.map(str::to_string);
            (row(id), facts)
        };
        let rows = [
            released("1", Some("v3")),
            released("2", None),
            released("3", Some("v1")),
            released("4", Some("v2")),
        ];
        let groups = run_with(&rows, GroupBy::Release, &HashMap::new(), &versions);

        assert_eq!(labels(&groups), ["1.1", "1.2", "2.0", "No release"]);
    }

    #[test]
    fn priority_groups_follow_the_level_with_no_priority_last() {
        let prioritised = |id: &str, name: Option<&str>| {
            let mut row = row(id);
            row.priority = name.map(priority);
            (row, facts())
        };
        let rows = [
            prioritised("1", None),
            prioritised("2", Some("Low")),
            prioritised("3", Some("High")),
        ];
        let groups = run(&rows, GroupBy::Priority);

        assert_eq!(labels(&groups), ["High", "Low", "No priority"]);
        assert_eq!(groups[0].mark, Some(GroupMark::Priority { level: 2 }));
    }
}
