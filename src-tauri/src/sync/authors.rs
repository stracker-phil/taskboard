//! Who made a recorded change (`issue_change.author_id`), from the issue's reporter, its new comments and its
//! changelog. Pure: no database, no clock.

use super::diff::ChangeRow;
use crate::jira::dto::Changelog;
use crate::time::utc_text;

/// One changelog entry: when, by whom, which fields (by id where Jira gives one).
pub struct Entry {
    pub at: String,
    pub author: Option<String>,
    pub fields: Vec<String>,
}

/// Entries with an unreadable time are left out.
pub fn entries_of(changelog: &[Changelog]) -> Vec<Entry> {
    changelog
        .iter()
        .filter_map(|entry| {
            Some(Entry {
                at: utc_text(&entry.created)?,
                author: entry.author.as_ref().map(|a| a.account_id.clone()),
                fields: entry
                    .items
                    .iter()
                    .map(|item| item.field_id.clone().unwrap_or_else(|| item.field.clone()))
                    .collect(),
            })
        })
        .collect()
}

/// The recorded fields (`issue_change.field`) the changelog can tell the author of.
pub const CHECKED_FIELDS: &[&str] = &[
    "status", "assignee", "priority", "due_date", "people", "updated",
];

enum Checked<'a> {
    /// A field by its changelog id.
    Field(&'a str),
    /// `updated`: any field since the owner last looked.
    Updated,
}

fn checked<'a>(field: &str, old_value: Option<&'a str>) -> Option<Checked<'a>> {
    Some(match field {
        "status" => Checked::Field("status"),
        "assignee" => Checked::Field("assignee"),
        "priority" => Checked::Field("priority"),
        "due_date" => Checked::Field("duedate"),
        // A people row keeps the field's id as its old value.
        "people" => Checked::Field(old_value?),
        "updated" => Checked::Updated,
        _ => return None,
    })
}

/// What an arriving issue tells about who made its changes.
pub struct Clues<'a> {
    pub reporter: Option<&'a str>,
    /// The authors of its fetched comments the cache didn't hold.
    pub new_comments: &'a [Option<String>],
    /// Its whole changelog, when the search brought all of it.
    pub history: Option<&'a [Entry]>,
    /// When the owner last looked at it; empty when never.
    pub seen: &'a str,
}

/// The change's author; none when the clues can't tell.
pub fn author_of(change: &ChangeRow, clues: &Clues) -> Option<String> {
    match change.field {
        "created" => clues.reporter.map(String::from),
        "comment_added" => comments_author(change.added_comments(), clues.new_comments),
        field => from_changelog(
            clues.history?,
            field,
            change.old_value.as_deref(),
            &change.changed_at,
            clues.seen,
        ),
    }
}

/// The one account that wrote every new comment, when the fetched ones cover all that were added.
fn comments_author(added: i64, new_comments: &[Option<String>]) -> Option<String> {
    if added <= 0 || (new_comments.len() as i64) < added {
        return None;
    }
    let first = new_comments.first()?.as_deref()?;
    new_comments
        .iter()
        .all(|a| a.as_deref() == Some(first))
        .then(|| first.to_string())
}

/// The changelog's answer for a recorded row; none for a field it doesn't name.
pub fn from_changelog(
    entries: &[Entry],
    field: &str,
    old_value: Option<&str>,
    changed_at: &str,
    seen: &str,
) -> Option<String> {
    match checked(field, old_value)? {
        Checked::Field(name) => last_change_by(entries, name, changed_at),
        Checked::Updated => single_author_since(entries, seen),
    }
    .map(String::from)
}

/// Who changed `field` last, up to `until`: the change that made the value the sync found.
fn last_change_by<'a>(entries: &'a [Entry], field: &str, until: &str) -> Option<&'a str> {
    entries
        .iter()
        .filter(|e| e.at.as_str() <= until && e.fields.iter().any(|f| f == field))
        .max_by(|a, b| a.at.cmp(&b.at))
        .and_then(|e| e.author.as_deref())
}

/// The one account behind every change after `seen`, when they all share one; none otherwise.
fn single_author_since<'a>(entries: &'a [Entry], seen: &str) -> Option<&'a str> {
    let mut authors = entries
        .iter()
        .filter(|e| e.at.as_str() > seen)
        .map(|e| e.author.as_deref());
    let first = authors.next()??;
    authors.all(|a| a == Some(first)).then_some(first)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const ALEX: &str = "account-alex";
    const XRAY: &str = "account-test-runner";
    const SEEN: &str = "2026-10-28T08:00:00.000Z";
    const AT: &str = "2026-10-28T10:00:00.000Z";

    fn entry(at: &str, author: Option<&str>, fields: &[&str]) -> Entry {
        Entry {
            at: at.into(),
            author: author.map(String::from),
            fields: fields.iter().map(|f| f.to_string()).collect(),
        }
    }

    fn row(field: &'static str, old: Option<&str>, new: Option<&str>) -> ChangeRow {
        ChangeRow {
            field,
            old_value: old.map(String::from),
            new_value: new.map(String::from),
            changed_at: AT.into(),
            author: None,
        }
    }

    fn comments(authors: &[Option<&str>]) -> Vec<Option<String>> {
        authors.iter().map(|a| a.map(String::from)).collect()
    }

    fn author(change: &ChangeRow, clues: &Clues) -> Option<String> {
        author_of(change, clues)
    }

    fn clues<'a>(history: Option<&'a [Entry]>, comments: &'a [Option<String>]) -> Clues<'a> {
        Clues {
            reporter: Some(ALEX),
            new_comments: comments,
            history,
            seen: SEEN,
        }
    }

    #[test]
    fn a_created_row_is_by_the_reporter() {
        let change = row("created", None, Some("NW-1"));

        assert_eq!(author(&change, &clues(None, &[])), Some(ALEX.into()));
    }

    #[test]
    fn a_created_row_without_a_reporter_has_no_author() {
        let change = row("created", None, Some("NW-1"));
        let clues = Clues {
            reporter: None,
            ..clues(None, &[])
        };

        assert_eq!(author(&change, &clues), None);
    }

    #[test]
    fn new_comments_by_one_account_are_by_that_account() {
        let change = row("comment_added", Some("1"), Some("3"));
        let fetched = comments(&[Some(XRAY), Some(XRAY)]);

        assert_eq!(author(&change, &clues(None, &fetched)), Some(XRAY.into()));
    }

    #[test]
    fn new_comments_are_judged_without_a_changelog() {
        let change = row("comment_added", Some("0"), Some("1"));
        let fetched = comments(&[Some(ALEX)]);

        assert_eq!(author(&change, &clues(None, &fetched)), Some(ALEX.into()));
    }

    #[test]
    fn new_comments_by_different_accounts_have_no_author() {
        let change = row("comment_added", Some("1"), Some("3"));
        let fetched = comments(&[Some(XRAY), Some(ALEX)]);

        assert_eq!(author(&change, &clues(None, &fetched)), None);
    }

    #[test]
    fn a_new_comment_without_a_known_author_has_no_author() {
        let change = row("comment_added", Some("1"), Some("2"));
        let fetched = comments(&[None]);

        assert_eq!(author(&change, &clues(None, &fetched)), None);
    }

    #[test]
    fn fewer_fetched_comments_than_added_have_no_author() {
        let change = row("comment_added", Some("1"), Some("4"));
        let fetched = comments(&[Some(XRAY), Some(XRAY)]);

        assert_eq!(author(&change, &clues(None, &fetched)), None);
    }

    #[test]
    fn a_comment_row_that_added_none_has_no_author() {
        for (old, new) in [("2", "2"), ("3", "2"), ("x", "y")] {
            let change = row("comment_added", Some(old), Some(new));
            let fetched = comments(&[Some(XRAY)]);

            assert_eq!(
                author(&change, &clues(None, &fetched)),
                None,
                "{old} -> {new}"
            );
        }
    }

    #[test]
    fn each_field_row_is_by_the_newest_entry_for_its_field_at_or_before_the_change() {
        let history = [
            entry(
                "2026-10-28T09:00:00.000Z",
                Some(ALEX),
                &["status", "assignee"],
            ),
            entry(
                "2026-10-28T09:30:00.000Z",
                Some(XRAY),
                &["status", "priority", "duedate"],
            ),
            entry(
                "2026-10-28T10:30:00.000Z",
                Some(ALEX),
                &["status", "priority", "duedate"],
            ),
        ];
        let no_comments: [Option<String>; 0] = [];
        let cases = [
            ("status", None, Some(XRAY)),
            ("assignee", None, Some(ALEX)),
            ("priority", None, Some(XRAY)),
            ("due_date", None, Some(XRAY)),
        ];
        for (field, old, expected) in cases {
            let change = row(field, old, None);

            assert_eq!(
                author(&change, &clues(Some(&history), &no_comments)),
                expected.map(String::from),
                "{field}"
            );
        }
    }

    #[test]
    fn an_entry_at_the_very_time_of_the_change_counts() {
        let history = [entry(AT, Some(XRAY), &["status"])];

        assert_eq!(
            author(&row("status", None, None), &clues(Some(&history), &[])),
            Some(XRAY.into())
        );
    }

    #[test]
    fn a_field_the_history_never_names_has_no_author() {
        let history = [entry("2026-10-28T09:00:00.000Z", Some(XRAY), &["priority"])];

        assert_eq!(
            author(&row("status", None, None), &clues(Some(&history), &[])),
            None
        );
    }

    #[test]
    fn only_entries_after_the_change_have_no_author() {
        let history = [entry("2026-10-28T10:30:00.000Z", Some(XRAY), &["status"])];

        assert_eq!(
            author(&row("status", None, None), &clues(Some(&history), &[])),
            None
        );
    }

    #[test]
    fn the_newest_entry_without_an_author_leaves_the_field_without_one() {
        let history = [
            entry("2026-10-28T09:00:00.000Z", Some(ALEX), &["status"]),
            entry("2026-10-28T09:30:00.000Z", None, &["status"]),
        ];

        assert_eq!(
            author(&row("status", None, None), &clues(Some(&history), &[])),
            None
        );
    }

    #[test]
    fn entries_are_found_whatever_their_order() {
        let history = [
            entry("2026-10-28T09:30:00.000Z", Some(XRAY), &["status"]),
            entry("2026-10-28T09:00:00.000Z", Some(ALEX), &["status"]),
        ];

        assert_eq!(
            author(&row("status", None, None), &clues(Some(&history), &[])),
            Some(XRAY.into())
        );
    }

    #[test]
    fn a_people_row_matches_by_the_field_id_it_keeps_as_its_old_value() {
        let history = [
            entry(
                "2026-10-28T09:00:00.000Z",
                Some(ALEX),
                &["customfield_10042"],
            ),
            entry(
                "2026-10-28T09:30:00.000Z",
                Some(XRAY),
                &["customfield_10043"],
            ),
        ];
        for (field_id, expected) in [
            ("customfield_10042", Some(ALEX)),
            ("customfield_10043", Some(XRAY)),
            ("customfield_10044", None),
        ] {
            let change = row("people", Some(field_id), Some("account-me"));

            assert_eq!(
                author(&change, &clues(Some(&history), &[])),
                expected.map(String::from),
                "{field_id}"
            );
        }
    }

    #[test]
    fn a_people_row_without_a_field_id_has_no_author() {
        let history = [entry(
            "2026-10-28T09:00:00.000Z",
            Some(ALEX),
            &["customfield_10042"],
        )];

        assert_eq!(
            author(
                &row("people", None, Some("account-me")),
                &clues(Some(&history), &[])
            ),
            None
        );
    }

    #[test]
    fn an_updated_row_is_by_the_single_author_of_every_entry_after_the_last_look() {
        let history = [
            entry("2026-10-28T07:00:00.000Z", Some(ALEX), &["status"]),
            entry("2026-10-28T09:00:00.000Z", Some(XRAY), &["status"]),
            entry("2026-10-28T09:30:00.000Z", Some(XRAY), &["labels"]),
        ];

        assert_eq!(
            author(&row("updated", None, None), &clues(Some(&history), &[])),
            Some(XRAY.into())
        );
    }

    #[test]
    fn an_updated_row_with_mixed_authors_since_the_last_look_has_no_author() {
        let history = [
            entry("2026-10-28T09:00:00.000Z", Some(XRAY), &["status"]),
            entry("2026-10-28T09:30:00.000Z", Some(ALEX), &["labels"]),
        ];

        assert_eq!(
            author(&row("updated", None, None), &clues(Some(&history), &[])),
            None
        );
    }

    #[test]
    fn an_updated_row_with_an_unknown_author_among_the_entries_has_no_author() {
        let history = [
            entry("2026-10-28T09:00:00.000Z", Some(XRAY), &["status"]),
            entry("2026-10-28T09:30:00.000Z", None, &["labels"]),
        ];

        assert_eq!(
            author(&row("updated", None, None), &clues(Some(&history), &[])),
            None
        );
    }

    #[test]
    fn an_updated_row_with_no_entry_since_the_last_look_has_no_author() {
        let history = [entry("2026-10-28T07:00:00.000Z", Some(ALEX), &["status"])];

        assert_eq!(
            author(&row("updated", None, None), &clues(Some(&history), &[])),
            None
        );
    }

    #[test]
    fn when_never_seen_every_entry_counts_for_an_updated_row() {
        let history = [
            entry("2026-10-01T07:00:00.000Z", Some(XRAY), &["status"]),
            entry("2026-10-28T09:00:00.000Z", Some(XRAY), &["labels"]),
        ];
        let clues = Clues {
            seen: "",
            ..clues(Some(&history), &[])
        };

        assert_eq!(
            author(&row("updated", None, None), &clues),
            Some(XRAY.into())
        );
    }

    #[test]
    fn without_a_history_field_rows_have_no_author_but_created_and_comments_still_do() {
        let fetched = comments(&[Some(XRAY)]);
        for field in ["status", "assignee", "priority", "due_date", "updated"] {
            assert_eq!(
                author(&row(field, None, None), &clues(None, &fetched)),
                None,
                "{field}"
            );
        }
        assert_eq!(
            author(
                &row("people", Some("customfield_10042"), None),
                &clues(None, &fetched)
            ),
            None
        );
    }

    #[test]
    fn a_field_the_changelog_cannot_tell_has_no_author() {
        let history = [entry(
            "2026-10-28T09:00:00.000Z",
            Some(XRAY),
            &["mention", "sprint"],
        )];

        for field in ["mention", "gone", "sprint"] {
            assert_eq!(
                from_changelog(&history, field, None, AT, SEEN),
                None,
                "{field}"
            );
        }
    }

    #[test]
    fn the_checked_fields_are_the_ones_the_changelog_answers_for() {
        let history = [entry(
            "2026-10-28T09:00:00.000Z",
            Some(XRAY),
            &[
                "status",
                "assignee",
                "priority",
                "duedate",
                "customfield_10042",
            ],
        )];

        for field in CHECKED_FIELDS {
            assert_eq!(
                from_changelog(&history, field, Some("customfield_10042"), AT, SEEN),
                Some(XRAY.into()),
                "{field}"
            );
        }
    }

    #[test]
    fn entries_carry_utc_times_authors_and_field_ids_with_the_name_as_fallback() {
        let changelog: Vec<Changelog> = serde_json::from_value(json!([
            {
                "id": "1",
                "author": {"accountId": XRAY, "displayName": "Test Runner", "accountType": "app"},
                "created": "2026-10-28T11:30:00.000+0200",
                "items": [
                    {"field": "status", "fieldId": "status"},
                    {"field": "Sprint"}
                ]
            },
            {"id": "2", "created": "2026-10-28T12:00:00.000+0200", "items": []},
            {"id": "3", "created": "not a time", "items": [{"field": "status"}]}
        ]))
        .unwrap();

        let entries = entries_of(&changelog);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].at, "2026-10-28T09:30:00.000Z");
        assert_eq!(entries[0].author.as_deref(), Some(XRAY));
        assert_eq!(entries[0].fields, ["status", "Sprint"]);
        assert_eq!(entries[1].at, "2026-10-28T10:00:00.000Z");
        assert_eq!(entries[1].author, None);
        assert!(entries[1].fields.is_empty());
    }
}
