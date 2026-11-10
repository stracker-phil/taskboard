//! The order of a view's rows and the text match of the narrowing.

use std::cmp::Ordering;

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use super::groups::Facts;
use super::model::SortBy;
use super::rows::Row;

/// Stable: equal rows keep their id order, so a refresh never reshuffles them. Rows without the sorted value (a
/// priority, a due date, a last view or interaction) sort last in either direction.
pub fn sort(items: &mut [(Row, Facts)], by: SortBy, descending: bool) {
    items.sort_by_key(|(row, _)| numeric_id(&row.id));
    items.sort_by(|(a, fa), (b, fb)| {
        let ordering = match by {
            SortBy::Updated => Some(fa.updated_at.cmp(&fb.updated_at)),
            SortBy::Created => Some(fa.created_at.cmp(&fb.created_at)),
            SortBy::Key => Some(key_order(&a.key).cmp(&key_order(&b.key))),
            SortBy::Summary => Some(a.summary.to_lowercase().cmp(&b.summary.to_lowercase())),
            SortBy::Status => Some(a.style.cmp(&b.style)),
            SortBy::Priority => both(priority_order(a), priority_order(b)),
            SortBy::Due => both(fa.due_date, fb.due_date),
            SortBy::LastView => both(fa.last_view.as_ref(), fb.last_view.as_ref()),
            SortBy::LastInteraction => {
                both(fa.last_interaction.as_ref(), fb.last_interaction.as_ref())
            }
        };
        match ordering {
            Some(ordering) if descending => ordering.reverse(),
            Some(ordering) => ordering,
            None => missing_last(by, a, fa, b, fb),
        }
    });
}

/// Higher priorities compare greater (level 1 is the highest).
fn priority_order(row: &Row) -> Option<std::cmp::Reverse<u8>> {
    row.priority.as_ref().map(|p| std::cmp::Reverse(p.level))
}

/// The comparison when both values are there.
fn both<T: Ord>(a: Option<T>, b: Option<T>) -> Option<Ordering> {
    Some(a?.cmp(&b?))
}

/// At least one value is missing: the row that has one comes first.
fn missing_last(by: SortBy, a: &Row, fa: &Facts, b: &Row, fb: &Facts) -> Ordering {
    let has = |row: &Row, facts: &Facts| match by {
        SortBy::Priority => row.priority.is_some(),
        SortBy::LastView => facts.last_view.is_some(),
        SortBy::LastInteraction => facts.last_interaction.is_some(),
        _ => facts.due_date.is_some(),
    };
    has(b, fb).cmp(&has(a, fa))
}

/// Malformed ids sort last.
fn numeric_id(id: &str) -> u64 {
    id.parse().unwrap_or(u64::MAX)
}

/// `NW-1029` → ("NW", 1029), so NW-99 sorts before NW-100; a malformed number sorts last in its project.
fn key_order(key: &str) -> (&str, u64) {
    match key.rsplit_once('-') {
        Some((project, number)) => (project, number.parse().unwrap_or(u64::MAX)),
        None => (key, u64::MAX),
    }
}

/// Lower case without accents ("Säule" → "saule").
pub fn fold(text: &str) -> String {
    text.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::status_style::Style;

    fn item(id: &str, last_view: Option<&str>, last_interaction: Option<&str>) -> (Row, Facts) {
        let row = Row {
            id: id.to_string(),
            key: format!("DEMO-{id}"),
            summary: format!("Task {id}"),
            style: Style::Progress,
            status_name: "In Progress".to_string(),
            comment_count: 0,
            assignees: Vec::new(),
            epic: None,
            release: None,
            priority: None,
            labels: Vec::new(),
            sprint: None,
            due: None,
            updated_at: String::new(),
            last_view: None,
            last_interaction: None,
            custom: BTreeMap::new(),
            child_count: 0,
            is_subtask: false,
            parent_id: None,
            depth: 0,
        };
        let facts = Facts {
            status_id: "1".to_string(),
            project_id: "p1".to_string(),
            assignee_id: None,
            release_id: None,
            due_date: None,
            updated_at: String::new(),
            created_at: String::new(),
            last_view: last_view.map(str::to_string),
            last_interaction: last_interaction.map(str::to_string),
        };
        (row, facts)
    }

    fn sorted_ids(items: &mut [(Row, Facts)], by: SortBy, descending: bool) -> Vec<String> {
        sort(items, by, descending);
        items.iter().map(|(row, _)| row.id.clone()).collect()
    }

    fn viewed_items() -> Vec<(Row, Facts)> {
        vec![
            item("1", None, None),
            item(
                "2",
                Some("2026-10-28T09:00:00.000Z"),
                Some("2026-10-28T12:00:00.000Z"),
            ),
            item("3", Some("2026-10-27T09:00:00.000Z"), None),
            item("4", None, Some("2026-10-26T12:00:00.000Z")),
            item(
                "5",
                Some("2026-10-28T15:00:00.000Z"),
                Some("2026-10-28T15:00:00.000Z"),
            ),
        ]
    }

    #[test]
    fn sorting_by_last_view_puts_issues_never_viewed_last_in_both_directions() {
        for (descending, expected) in [
            (false, ["3", "2", "5", "1", "4"]),
            (true, ["5", "2", "3", "1", "4"]),
        ] {
            assert_eq!(
                sorted_ids(&mut viewed_items(), SortBy::LastView, descending),
                expected,
                "descending: {descending}"
            );
        }
    }

    #[test]
    fn sorting_by_last_interaction_puts_untouched_issues_last_in_both_directions() {
        for (descending, expected) in [
            (false, ["4", "2", "5", "1", "3"]),
            (true, ["5", "2", "4", "1", "3"]),
        ] {
            assert_eq!(
                sorted_ids(&mut viewed_items(), SortBy::LastInteraction, descending),
                expected,
                "descending: {descending}"
            );
        }
    }

    #[test]
    fn issues_with_the_same_last_view_keep_their_id_order() {
        let mut items = vec![
            item("12", Some("2026-10-28T09:00:00.000Z"), None),
            item("7", Some("2026-10-28T09:00:00.000Z"), None),
            item("9", None, None),
        ];

        assert_eq!(
            sorted_ids(&mut items, SortBy::LastView, true),
            ["7", "12", "9"]
        );
    }

    #[test]
    fn folding_lowercases_and_drops_accents() {
        for (input, expected) in [
            ("Säule", "saule"),
            ("ÉCOLE", "ecole"),
            ("naïve Café", "naive cafe"),
        ] {
            assert_eq!(fold(input), expected, "{input}");
        }
    }
}
