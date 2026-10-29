//! One issue as a list row shows it: everything is resolved and formatted here, so the page only draws.

use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use serde_json::Value as Json;
use ts_rs::TS;

use crate::status_style::Style;

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub key: String,
    pub summary: String,
    pub style: Style,
    pub status_name: String,
    pub comment_count: i64,
    /// Changed since the owner last looked.
    pub changed: bool,
    /// The change chip's text ("2 new comments").
    pub change_note: Option<String>,
    pub assignees: Vec<Person>,
    pub epic: Option<EpicChip>,
    pub release: Option<String>,
    pub priority: Option<Priority>,
    pub labels: Vec<Label>,
    pub sprint: Option<String>,
    /// The due date as the list shows it ("Oct 30").
    pub due: Option<String>,
    /// Custom column values by field id; a field without a value is absent.
    pub custom: BTreeMap<String, Cell>,
    /// All its subtasks in the cache, whatever the view's filter.
    pub subtask_count: i64,
    pub is_subtask: bool,
    /// A subtask's parent; none on other issues.
    pub parent_id: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub account_id: String,
    pub name: String,
    pub initials: String,
    /// The `--avatar-N` token.
    pub color: u8,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct EpicChip {
    pub id: String,
    pub name: String,
    /// The `--epic-N-*` token pair.
    pub color: u8,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Priority {
    pub name: String,
    /// 1 Highest … 5 Lowest; an unknown name counts as Medium.
    pub level: u8,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub name: String,
    /// The `--label-N-*` token pair; 0 is the neutral one.
    pub tint: u8,
}

#[derive(Serialize, Clone, Debug, PartialEq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Cell {
    Text(String),
    Person(Person),
}

const AVATAR_COLORS: u8 = 4;
const EPIC_COLORS: u8 = 5;
const LABEL_TINTS: u8 = 3;

/// FNV-1a: stable across runs and platforms, unlike the std hasher.
fn stable_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// `me` is always the first avatar color; everyone else gets one of the others.
pub fn person(account_id: &str, name: &str, me: Option<&str>) -> Person {
    let color = if me == Some(account_id) {
        1
    } else {
        2 + (stable_hash(account_id) % u64::from(AVATAR_COLORS - 1)) as u8
    };
    Person {
        account_id: account_id.to_string(),
        name: name.to_string(),
        initials: initials(name),
        color,
    }
}

/// First and last word's first letters ("Alex Morgan" → "AM"); one word gives one letter.
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let letters = match words.as_slice() {
        [] => Vec::new(),
        [only] => vec![*only],
        [first, .., last] => vec![*first, *last],
    };
    letters
        .iter()
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

/// Jira's epic color (`ghx-label-N`, 1–14) mapped to the nearest of the five epic tints; an epic without one
/// gets a tint from its id.
pub fn epic_color(ghx_label: Option<&str>, epic_id: &str) -> u8 {
    let jira = ghx_label
        .and_then(|label| label.strip_prefix("ghx-label-"))
        .and_then(|n| n.parse::<u8>().ok());
    /* Jira's labels by tint: purple (7, 8, 12); brown, orange, yellow (1–3); blues and slates (4, 5, 10, 11);
    greens (6, 13); red, rose (9, 14). */
    match jira {
        Some(7 | 8 | 12) => 1,
        Some(1..=3) => 2,
        Some(4 | 5 | 10 | 11) => 3,
        Some(6 | 13) => 4,
        Some(9 | 14) => 5,
        _ => 1 + (stable_hash(epic_id) % u64::from(EPIC_COLORS)) as u8,
    }
}

pub fn priority(name: &str) -> Priority {
    let level = match name.to_lowercase().as_str() {
        "highest" | "blocker" => 1,
        "high" | "critical" => 2,
        "low" | "minor" => 4,
        "lowest" | "trivial" => 5,
        _ => 3,
    };
    Priority {
        name: name.to_string(),
        level,
    }
}

pub fn label(name: &str) -> Label {
    Label {
        name: name.to_string(),
        tint: (stable_hash(name) % u64::from(LABEL_TINTS)) as u8,
    }
}

/// "Oct 30", with the year when it isn't this year's.
pub fn format_due(due: NaiveDate, today: NaiveDate) -> String {
    if due.year() == today.year() {
        due.format("%b %-d").to_string()
    } else {
        due.format("%b %-d, %Y").to_string()
    }
}

/// A custom field's JSON value as a cell, by its Jira schema type.
pub fn cell(kind: &str, value: &Json, me: Option<&str>) -> Option<Cell> {
    match (kind, value) {
        (_, Json::Null) => None,
        ("user", Json::Object(user)) => {
            let id = user.get("accountId")?.as_str()?;
            let name = user.get("displayName").and_then(Json::as_str).unwrap_or(id);
            Some(Cell::Person(person(id, name, me)))
        }
        (_, Json::Array(items)) => {
            let names: Vec<String> = items.iter().filter_map(display_text).collect();
            (!names.is_empty()).then(|| Cell::Text(names.join(", ")))
        }
        _ => display_text(value).map(Cell::Text),
    }
}

/// Text of a scalar, or of an option or object by its `value`, `name` or `displayName`.
fn display_text(value: &Json) -> Option<String> {
    match value {
        Json::String(text) if !text.is_empty() => Some(text.clone()),
        Json::Number(number) => Some(number.to_string()),
        Json::Bool(flag) => Some(if *flag { "Yes" } else { "No" }.to_string()),
        Json::Object(object) => ["value", "name", "displayName"]
            .iter()
            .find_map(|member| object.get(*member).and_then(display_text)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn date(text: &str) -> NaiveDate {
        text.parse().unwrap()
    }

    #[test]
    fn initials_are_the_first_and_last_words_first_letters() {
        for (name, expected) in [
            ("Alex Morgan", "AM"),
            ("Ana Maria de Souza", "AS"),
            ("Cher", "C"),
            ("", ""),
            ("lea novak", "LN"),
        ] {
            assert_eq!(initials(name), expected, "{name:?}");
        }
    }

    #[test]
    fn me_gets_the_first_avatar_color_and_others_a_stable_one_of_the_rest() {
        let me = person("acc-me", "Alex Morgan", Some("acc-me"));
        assert_eq!(me.color, 1);
        assert_eq!(me.initials, "AM");
        assert_eq!(me.account_id, "acc-me");
        assert_eq!(me.name, "Alex Morgan");

        for id in ["acc-1", "acc-2", "acc-3", "acc-4", "acc-5", "acc-6"] {
            let first = person(id, "Lea Novak", Some("acc-me"));
            assert!((2..=4).contains(&first.color), "{id}");
            assert_eq!(first, person(id, "Lea Novak", Some("acc-me")), "{id}");
        }
    }

    #[test]
    fn jira_epic_colors_map_to_the_five_tints() {
        for (ghx, expected) in [
            ("ghx-label-7", 1),
            ("ghx-label-3", 2),
            ("ghx-label-4", 3),
            ("ghx-label-13", 4),
            ("ghx-label-9", 5),
        ] {
            assert_eq!(epic_color(Some(ghx), "10042"), expected, "{ghx}");
        }
    }

    #[test]
    fn an_epic_without_a_known_color_gets_a_stable_tint_from_its_id() {
        for ghx in [None, Some("red"), Some("ghx-label-99")] {
            let color = epic_color(ghx, "10042");
            assert!((1..=5).contains(&color), "{ghx:?}");
            assert_eq!(color, epic_color(ghx, "10042"), "{ghx:?}");
        }
    }

    #[test]
    fn priority_levels_follow_the_name_case_insensitively() {
        for (name, level) in [
            ("Highest", 1),
            ("High", 2),
            ("Medium", 3),
            ("Low", 4),
            ("LOWEST", 5),
            ("highest", 1),
            ("Urgent-ish", 3),
        ] {
            assert_eq!(
                priority(name),
                Priority {
                    name: name.to_string(),
                    level
                },
                "{name}"
            );
        }
    }

    #[test]
    fn a_label_has_a_stable_tint_in_range() {
        for name in ["backend", "design", "needs-info"] {
            let tint = label(name).tint;
            assert!(tint <= 2, "{name}");
            assert_eq!(label(name), label(name), "{name}");
            assert_eq!(label(name).name, name);
        }
    }

    #[test]
    fn due_dates_show_the_year_only_when_it_is_not_this_year() {
        let today = date("2026-10-28");
        for (due, expected) in [
            ("2026-10-30", "Oct 30"),
            ("2026-03-04", "Mar 4"),
            ("2027-01-04", "Jan 4, 2027"),
            ("2025-12-31", "Dec 31, 2025"),
        ] {
            assert_eq!(format_due(date(due), today), expected, "{due}");
        }
    }

    #[test]
    fn a_user_value_becomes_a_person_cell() {
        let value = json!({ "accountId": "acc-9", "displayName": "Lea Novak" });
        assert_eq!(
            cell("user", &value, None),
            Some(Cell::Person(person("acc-9", "Lea Novak", None)))
        );
    }

    #[test]
    fn scalar_and_option_values_become_text_cells() {
        for (value, expected) in [
            (json!(5), "5"),
            (json!("Plain"), "Plain"),
            (json!({ "value": "Large" }), "Large"),
            (json!(true), "Yes"),
            (json!(false), "No"),
            (json!(["a", "b"]), "a, b"),
            (
                json!([{ "name": "Web" }, { "name": "Mobile" }]),
                "Web, Mobile",
            ),
        ] {
            assert_eq!(
                cell("string", &value, None),
                Some(Cell::Text(expected.to_string())),
                "{value}"
            );
        }
    }

    #[test]
    fn empty_values_make_no_cell() {
        for value in [json!(null), json!(""), json!([])] {
            assert_eq!(cell("string", &value, None), None, "{value}");
        }
    }
}
