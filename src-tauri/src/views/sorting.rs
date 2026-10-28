//! The order of a view's rows and the text match of the narrowing.

use std::cmp::Ordering;

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use super::groups::Facts;
use super::model::SortBy;
use super::rows::Row;

/// Stable: equal rows keep their id order, so a refresh never reshuffles them. Rows without a priority or a due
/// date sort last in either direction.
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
    use super::*;

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
