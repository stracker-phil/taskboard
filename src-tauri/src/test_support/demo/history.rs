//! The changelogs of a few demo issues: status moves, reassignments, priority, due date, fix version, sprint,
//! label, summary and description changes, shaped like `GET /issue/{id}/changelog` `values`.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::catalog::{Priority, SPRINTS, Status, version};
use super::clock::Clock;
use super::people::{AM, JR, MK, Person, user};
use super::render::issue_id;
use Item::*;

const FIRST_ENTRY_ID: u64 = 80_001;

/// One field's change, before it becomes a Jira item.
enum Item {
    Moved(Status, Status),
    Assigned(Option<&'static Person>, Option<&'static Person>),
    Reprioritized(Priority, Priority),
    Due(Option<&'static str>, &'static str),
    FixVersion(&'static str, &'static str),
    /// Sprint ids before and after.
    Sprint(&'static [u64], &'static [u64]),
    Labels(&'static str, &'static str),
    Summary(&'static str, &'static str),
    Description(&'static str, &'static str),
}

struct Entry {
    issue: &'static str,
    author: &'static Person,
    /// Berlin wall-clock time, as in the issue tables.
    at: &'static str,
    items: &'static [Item],
}

const POSTCODES_BEFORE: &str = "Validate postcodes in the checkout address form.";
const POSTCODES_AFTER: &str = "Validate postcodes for Germany, Austria and Switzerland in the checkout address \
    form, with inline errors that name the expected format for the chosen country.";

#[rustfmt::skip]
const ENTRIES: &[Entry] = &[
    Entry { issue: "NW-1042", author: JR, at: "2026-10-12 09:20", items: &[Reprioritized(Priority::Medium, Priority::High)] },
    Entry { issue: "NW-1042", author: JR, at: "2026-10-13 09:05", items: &[Assigned(None, Some(AM)), Due(None, "2026-11-06")] },
    Entry { issue: "NW-1042", author: AM, at: "2026-10-14 10:10", items: &[FixVersion("", "2.4.0"), Labels("", "frontend")] },
    Entry { issue: "NW-1042", author: AM, at: "2026-10-15 14:00", items: &[Description(POSTCODES_BEFORE, POSTCODES_AFTER)] },
    Entry { issue: "NW-1042", author: JR, at: "2026-10-20 11:00", items: &[Due(Some("2026-11-06"), "2026-10-30")] },
    Entry { issue: "NW-1042", author: JR, at: "2026-10-22 16:30", items: &[Sprint(&[], &[22])] },
    Entry { issue: "NW-1042", author: AM, at: "2026-10-23 09:30", items: &[Moved(Status::ToDo, Status::InProgress)] },
    Entry { issue: "NW-1042", author: AM, at: "2026-10-26 17:50", items: &[Moved(Status::InProgress, Status::InReview)] },
    Entry { issue: "NW-1042", author: MK, at: "2026-10-28 11:41", items: &[Moved(Status::InReview, Status::InProgress)] },
    Entry { issue: "NW-1038", author: MK, at: "2026-10-07 11:30", items: &[Reprioritized(Priority::High, Priority::Highest)] },
    Entry { issue: "NW-1038", author: MK, at: "2026-10-08 09:20", items: &[Assigned(None, Some(JR))] },
    Entry { issue: "NW-1038", author: JR, at: "2026-10-15 11:45", items: &[Assigned(Some(JR), Some(AM))] },
    Entry { issue: "NW-1038", author: JR, at: "2026-10-23 09:10", items: &[Sprint(&[21], &[21, 22])] },
    Entry { issue: "NW-1038", author: AM, at: "2026-10-27 09:00", items: &[Moved(Status::ToDo, Status::InProgress)] },
    Entry { issue: "NW-1038", author: AM, at: "2026-10-28 09:28", items: &[Moved(Status::InProgress, Status::InReview)] },
    Entry { issue: "NW-1047", author: JR, at: "2026-10-14 13:25", items: &[Assigned(None, Some(AM))] },
    Entry { issue: "NW-1047", author: AM, at: "2026-10-21 10:00", items: &[Moved(Status::ToDo, Status::InProgress)] },
    Entry { issue: "NW-1047", author: AM, at: "2026-10-22 15:00", items: &[Summary("Cart totals via Store API", "Migrate cart totals to the Store API")] },
    Entry { issue: "NW-1033", author: AM, at: "2026-10-05 09:00", items: &[Moved(Status::ToDo, Status::InProgress)] },
    Entry { issue: "NW-1033", author: JR, at: "2026-10-12 11:00", items: &[Due(Some("2026-10-16"), "2026-10-27")] },
    Entry { issue: "NW-1033", author: AM, at: "2026-10-20 10:10", items: &[Moved(Status::InProgress, Status::Blocked)] },
];

/// Issue id to its changelog entries, oldest first.
pub fn changelogs(clock: &Clock) -> BTreeMap<String, Vec<Value>> {
    let mut by_issue: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for (offset, entry) in (0_u64..).zip(ENTRIES) {
        let items: Vec<Value> = entry
            .items
            .iter()
            .map(|item| item_json(item, clock))
            .collect();
        by_issue
            .entry(issue_id(entry.issue))
            .or_default()
            .push(json!({
                "id": (FIRST_ENTRY_ID + offset).to_string(),
                "author": user(entry.author),
                "created": clock.stamp(entry.at),
                "items": items,
            }));
    }
    by_issue
}

fn item_json(item: &Item, clock: &Clock) -> Value {
    let id_name = |value: Value| (value["id"].clone(), value["name"].clone());
    match item {
        Moved(from, to) => {
            let ((from_id, from_name), (to_id, to_name)) =
                (id_name(from.json()), id_name(to.json()));
            change(
                "status", "status", "jira", from_id, from_name, to_id, to_name,
            )
        }
        Assigned(from, to) => {
            let id = |p: &Option<&Person>| json!(p.map(|p| p.account_id));
            let name = |p: &Option<&Person>| json!(p.map(|p| p.name));
            change(
                "assignee",
                "assignee",
                "jira",
                id(from),
                name(from),
                id(to),
                name(to),
            )
        }
        Reprioritized(from, to) => {
            let ((from_id, from_name), (to_id, to_name)) =
                (id_name(from.json()), id_name(to.json()));
            change(
                "priority", "priority", "jira", from_id, from_name, to_id, to_name,
            )
        }
        Due(from, to) => {
            let date = |d: &str| clock.date(d);
            let from = from.map(date);
            let to = date(to);
            change(
                "duedate",
                "duedate",
                "jira",
                json!(from),
                json!(from.as_ref().map(|d| format!("{d} 00:00:00.0"))),
                json!(to),
                json!(format!("{to} 00:00:00.0")),
            )
        }
        FixVersion(from, to) => {
            let id = |name: &str| json!((!name.is_empty()).then(|| version("NW", name).id));
            let name = |name: &str| json!((!name.is_empty()).then_some(name));
            change(
                "Fix Version",
                "fixVersions",
                "jira",
                id(from),
                name(from),
                id(to),
                name(to),
            )
        }
        Sprint(from, to) => {
            let ids = |list: &[u64]| {
                let joined = list
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                json!((!joined.is_empty()).then_some(joined))
            };
            let names = |list: &[u64]| {
                let names: Vec<&str> = list
                    .iter()
                    .filter_map(|id| SPRINTS.iter().find(|s| s.id == *id).map(|s| s.name))
                    .collect();
                json!(names.join(", "))
            };
            change(
                "Sprint",
                "customfield_10020",
                "custom",
                ids(from),
                names(from),
                ids(to),
                names(to),
            )
        }
        Labels(from, to) => free_text("labels", from, to),
        Summary(from, to) => free_text("summary", from, to),
        Description(from, to) => free_text("description", from, to),
    }
}

/// A field without ids: only its texts change.
fn free_text(field: &str, from: &str, to: &str) -> Value {
    change(
        field,
        field,
        "jira",
        Value::Null,
        json!(from),
        Value::Null,
        json!(to),
    )
}

fn change(
    field: &str,
    field_id: &str,
    fieldtype: &str,
    from: Value,
    from_text: Value,
    to: Value,
    to_text: Value,
) -> Value {
    json!({
        "field": field,
        "fieldtype": fieldtype,
        "fieldId": field_id,
        "from": from,
        "fromString": from_text,
        "to": to,
        "toString": to_text,
    })
}
