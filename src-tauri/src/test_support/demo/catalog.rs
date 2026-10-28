//! The demo site's projects, statuses, issue types, priorities, versions and sprints.

use serde_json::{Value, json};

use super::SITE_URL;
use super::clock::Clock;

pub struct Project {
    pub id: &'static str,
    pub key: &'static str,
    pub name: &'static str,
    /// Leading digit of the project's issue ids, so ids stay stable when the tables grow.
    pub id_prefix: u64,
}

#[rustfmt::skip]
pub const PROJECTS: [Project; 6] = [
    Project { id: "10001", key: "NW", name: "Website relaunch", id_prefix: 1 },
    Project { id: "10002", key: "NWS", name: "Northwind support", id_prefix: 2 },
    Project { id: "10003", key: "LUM", name: "Lumen iOS app", id_prefix: 3 },
    Project { id: "10004", key: "FBA", name: "Fernweh booking API", id_prefix: 4 },
    Project { id: "10005", key: "INT", name: "Internal tools", id_prefix: 5 },
    Project { id: "10006", key: "ARC", name: "Archive 2024", id_prefix: 6 },
];

pub fn project(key: &str) -> &'static Project {
    PROJECTS
        .iter()
        .find(|p| p.key == key)
        .expect("demo project")
}

fn avatars(kind: &str, id: &str) -> Value {
    let url = |size: u32| format!("https://avatar.example.com/{kind}/{id}/{size}.png");
    json!({ "48x48": url(48), "24x24": url(24), "16x16": url(16), "32x32": url(32) })
}

/// The short project object embedded in issues.
pub fn project_ref(project: &Project) -> Value {
    json!({
        "self": format!("{SITE_URL}/rest/api/3/project/{}", project.id),
        "id": project.id,
        "key": project.key,
        "name": project.name,
        "projectTypeKey": "software",
        "simplified": false,
        "avatarUrls": avatars("project", project.id),
    })
}

/// A `GET /project/search` item.
pub fn projects() -> Vec<Value> {
    PROJECTS
        .iter()
        .map(|project| {
            let mut value = project_ref(project);
            value["expand"] =
                json!("description,lead,issueTypes,url,projectKeys,permissions,insight");
            value["style"] = json!("classic");
            value["isPrivate"] = json!(false);
            value["properties"] = json!({});
            value
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    ToDo,
    InProgress,
    InReview,
    Blocked,
    Done,
}

impl Status {
    pub const ALL: [Status; 5] = [
        Status::ToDo,
        Status::InProgress,
        Status::InReview,
        Status::Blocked,
        Status::Done,
    ];

    pub fn is_done(self) -> bool {
        self == Status::Done
    }

    pub fn json(self) -> Value {
        let (id, name) = match self {
            Status::ToDo => ("10000", "To Do"),
            Status::InProgress => ("3", "In Progress"),
            Status::InReview => ("10002", "In Review"),
            Status::Blocked => ("10003", "Blocked"),
            Status::Done => ("10001", "Done"),
        };
        let (category_id, key, color, category) = match self {
            Status::ToDo => (2, "new", "blue-gray", "To Do"),
            Status::Done => (3, "done", "green", "Done"),
            _ => (4, "indeterminate", "yellow", "In Progress"),
        };
        json!({
            "self": format!("{SITE_URL}/rest/api/3/status/{id}"),
            "description": "",
            "iconUrl": format!("{SITE_URL}/images/icons/statuses/generic.png"),
            "name": name,
            "id": id,
            "statusCategory": {
                "self": format!("{SITE_URL}/rest/api/3/statuscategory/{category_id}"),
                "id": category_id,
                "key": key,
                "colorName": color,
                "name": category,
            },
        })
    }
}

pub fn statuses() -> Vec<Value> {
    Status::ALL.iter().map(|status| status.json()).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Epic,
    Story,
    Task,
    Bug,
    Subtask,
}

impl Kind {
    pub fn json(self) -> Value {
        let (id, name, level) = match self {
            Kind::Epic => ("10000", "Epic", 1),
            Kind::Story => ("10001", "Story", 0),
            Kind::Task => ("10002", "Task", 0),
            Kind::Bug => ("10003", "Bug", 0),
            Kind::Subtask => ("10004", "Subtask", -1),
        };
        json!({
            "self": format!("{SITE_URL}/rest/api/3/issuetype/{id}"),
            "id": id,
            "description": "",
            "iconUrl": format!("{SITE_URL}/rest/api/2/universal_avatar/view/type/issuetype/avatar/{id}"),
            "name": name,
            "subtask": self == Kind::Subtask,
            "hierarchyLevel": level,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    Highest,
    High,
    Medium,
    Low,
    Lowest,
}

impl Priority {
    pub const ALL: [Priority; 5] = [
        Priority::Highest,
        Priority::High,
        Priority::Medium,
        Priority::Low,
        Priority::Lowest,
    ];

    pub fn json(self) -> Value {
        let (id, name) = match self {
            Priority::Highest => ("1", "Highest"),
            Priority::High => ("2", "High"),
            Priority::Medium => ("3", "Medium"),
            Priority::Low => ("4", "Low"),
            Priority::Lowest => ("5", "Lowest"),
        };
        json!({
            "self": format!("{SITE_URL}/rest/api/3/priority/{id}"),
            "iconUrl": format!("{SITE_URL}/images/icons/priorities/{}.svg", name.to_lowercase()),
            "name": name,
            "id": id,
        })
    }
}

pub struct Version {
    pub id: &'static str,
    pub project: &'static str,
    pub name: &'static str,
    pub released: bool,
    pub release_date: Option<&'static str>,
}

#[rustfmt::skip]
pub const VERSIONS: [Version; 6] = [
    Version { id: "10100", project: "NW", name: "2.3.1", released: true, release_date: Some("2026-10-02") },
    Version { id: "10101", project: "NW", name: "2.4.0", released: false, release_date: Some("2026-11-13") },
    Version { id: "10102", project: "NW", name: "2.5.0", released: false, release_date: Some("2026-12-11") },
    Version { id: "10110", project: "LUM", name: "3.2.0", released: true, release_date: Some("2026-10-15") },
    Version { id: "10111", project: "LUM", name: "3.3.0", released: false, release_date: Some("2026-11-20") },
    Version { id: "10112", project: "LUM", name: "4.0", released: false, release_date: None },
];

pub fn version(project: &str, name: &str) -> &'static Version {
    VERSIONS
        .iter()
        .find(|v| v.project == project && v.name == name)
        .expect("demo version")
}

/// A `GET /project/{key}/versions` item; issues embed the same object in `fixVersions`.
pub fn version_json(version: &Version, clock: &Clock) -> Value {
    let mut value = json!({
        "self": format!("{SITE_URL}/rest/api/3/version/{}", version.id),
        "id": version.id,
        "description": "",
        "name": version.name,
        "archived": false,
        "released": version.released,
        "projectId": project(version.project).id.parse::<u64>().expect("numeric project id"),
    });
    if let Some(date) = version.release_date {
        value["releaseDate"] = json!(clock.date(date));
    }
    value
}

pub struct Sprint {
    pub id: u64,
    pub name: &'static str,
    pub state: &'static str,
    pub board: u64,
    pub start: &'static str,
    pub end: &'static str,
}

#[rustfmt::skip]
pub const SPRINTS: [Sprint; 5] = [
    Sprint { id: 21, name: "Sprint 21", state: "closed", board: 1, start: "2026-10-09 09:00", end: "2026-10-23 17:00" },
    Sprint { id: 22, name: "Sprint 22", state: "active", board: 1, start: "2026-10-23 09:00", end: "2026-11-06 17:00" },
    Sprint { id: 23, name: "Sprint 23", state: "future", board: 1, start: "2026-11-06 09:00", end: "2026-11-20 17:00" },
    Sprint { id: 37, name: "LUM Sprint 8", state: "closed", board: 2, start: "2026-10-05 09:00", end: "2026-10-19 17:00" },
    Sprint { id: 38, name: "LUM Sprint 9", state: "active", board: 2, start: "2026-10-19 09:00", end: "2026-11-02 17:00" },
];

/// A `customfield_10020` entry, the shape Jira Software gives the Sprint field.
pub fn sprint_json(id: u64, clock: &Clock) -> Value {
    let sprint = SPRINTS.iter().find(|s| s.id == id).expect("demo sprint");
    let mut value = json!({
        "id": sprint.id,
        "name": sprint.name,
        "state": sprint.state,
        "boardId": sprint.board,
        "goal": "",
        "startDate": clock.planned_utc(sprint.start),
        "endDate": clock.planned_utc(sprint.end),
    });
    if sprint.state == "closed" {
        value["completeDate"] = json!(clock.planned_utc(sprint.end));
    }
    value
}

/// Taskboard's local Space settings for the demo projects. Not Jira data; a hint for the seed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceHint {
    pub key: &'static str,
    pub client: Option<&'static str>,
    /// Palette token, never a color literal.
    pub color: Option<&'static str>,
    pub icon: &'static str,
    pub tracked: bool,
}

pub fn demo_spaces() -> Vec<SpaceHint> {
    let space = |key, client, color, icon, tracked| SpaceHint {
        key,
        client,
        color,
        icon,
        tracked,
    };
    vec![
        space("NW", Some("Northwind"), Some("space-tile-1"), "W", true),
        space("NWS", Some("Northwind"), Some("space-tile-2"), "S", true),
        space("LUM", Some("Lumen"), Some("space-tile-3"), "L", true),
        space("FBA", Some("Fernweh"), Some("space-tile-4"), "F", true),
        space("INT", None, Some("space-tile-5"), "I", true),
        space("ARC", None, None, "A", false),
    ]
}
