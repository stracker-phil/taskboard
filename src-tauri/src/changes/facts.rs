//! Unseen changes in words: the change chip on a row and the Since-yesterday sentence.

use std::collections::HashMap;

use super::Change;
use crate::views::rows;

/// What a run of changes amounts to; each kind once, with its latest value.
#[derive(Debug, Clone, PartialEq)]
pub enum Fact {
    New,
    Comments(i64),
    /// To this status id.
    Moved(String),
    PriorityRaised,
    PriorityLowered,
    /// To this account id; none when unassigned.
    Assigned(Option<String>),
    Due,
}

/// The ids in the facts resolved to names.
pub struct Names<'a> {
    pub me: Option<&'a str>,
    pub statuses: &'a HashMap<String, String>,
    pub people: &'a HashMap<String, String>,
}

/// No priority ranks below Lowest.
const UNRANKED: u8 = 6;

fn comment_count(value: &Option<String>) -> i64 {
    value.as_deref().and_then(|n| n.parse().ok()).unwrap_or(0)
}

/// An unassigned issue is never "assigned to you", even when the connected account is unknown.
fn is_me(assignee: &Option<String>, me: Option<&str>) -> bool {
    assignee.is_some() && assignee.as_deref() == me
}

fn of_field<'a>(changes: &'a [Change], field: &'a str) -> impl Iterator<Item = &'a Change> {
    changes.iter().filter(move |c| c.field == field)
}

/// The facts, most important first.
pub fn facts(changes: &[Change]) -> Vec<Fact> {
    let of = |field| of_field(changes, field);
    let mut facts = Vec::new();
    if of("created").next().is_some() {
        facts.push(Fact::New);
    }
    let comments: i64 = of("comment_added")
        .map(|c| comment_count(&c.new_value) - comment_count(&c.old_value))
        .sum();
    if comments > 0 {
        facts.push(Fact::Comments(comments));
    }
    if let Some(status) = of("status").last().and_then(|c| c.new_value.clone()) {
        facts.push(Fact::Moved(status));
    }
    let level = |v: &Option<String>| {
        v.as_deref()
            .map_or(UNRANKED, |name| rows::priority(name).level)
    };
    if let (Some(first), Some(last)) = (of("priority").next(), of("priority").last()) {
        let (from, to) = (level(&first.old_value), level(&last.new_value));
        if from != to {
            facts.push(if to < from {
                Fact::PriorityRaised
            } else {
                Fact::PriorityLowered
            });
        }
    }
    if let Some(assignee) = of("assignee").last() {
        facts.push(Fact::Assigned(assignee.new_value.clone()));
    }
    if of("due_date").next().is_some() {
        facts.push(Fact::Due);
    }
    facts
}

fn status<'a>(names: &'a Names, id: &'a str) -> &'a str {
    names.statuses.get(id).map_or(id, String::as_str)
}

fn comments(n: i64) -> String {
    if n == 1 {
        "1 new comment".into()
    } else {
        format!("{n} new comments")
    }
}

/// The row's chip: the most important fact, and how many more there are ("Moved to Review +1").
pub fn chip(facts: &[Fact], names: &Names) -> Option<String> {
    let text = match facts.first()? {
        Fact::New => "New".into(),
        Fact::Comments(n) => comments(*n),
        Fact::Moved(id) => format!("Moved to {}", status(names, id)),
        Fact::PriorityRaised => "Priority raised".into(),
        Fact::PriorityLowered => "Priority lowered".into(),
        Fact::Assigned(to) if is_me(to, names.me) => "Assigned to you".into(),
        Fact::Assigned(Some(_)) => "Reassigned".into(),
        Fact::Assigned(None) => "Unassigned".into(),
        Fact::Due => "Due date changed".into(),
    };
    Some(match facts.len() {
        1 => text,
        n => format!("{text} +{}", n - 1),
    })
}

/// Every fact as a clause of the banner's sentence ("2 new comments, moved to Review.").
pub fn sentence(facts: &[Fact], names: &Names) -> Option<String> {
    let clauses: Vec<String> = facts
        .iter()
        .map(|fact| match fact {
            Fact::New => "created".into(),
            Fact::Comments(n) => comments(*n),
            Fact::Moved(id) => format!("moved to {}", status(names, id)),
            Fact::PriorityRaised => "priority raised".into(),
            Fact::PriorityLowered => "priority lowered".into(),
            Fact::Assigned(to) if is_me(to, names.me) => "assigned to you".into(),
            Fact::Assigned(Some(id)) => {
                format!(
                    "reassigned to {}",
                    names.people.get(id).map_or(id.as_str(), String::as_str)
                )
            }
            Fact::Assigned(None) => "unassigned".into(),
            Fact::Due => "due date changed".into(),
        })
        .collect();
    (!clauses.is_empty()).then(|| format!("{}.", clauses.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "account-me";
    const MARTA: &str = "account-marta";

    fn change(field: &str, old: Option<&str>, new: Option<&str>) -> Change {
        Change {
            field: field.into(),
            old_value: old.map(String::from),
            new_value: new.map(String::from),
            at: "2026-10-28T09:00:00.000Z".into(),
        }
    }

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn names<'a>(
        me: Option<&'a str>,
        statuses: &'a HashMap<String, String>,
        people: &'a HashMap<String, String>,
    ) -> Names<'a> {
        Names {
            me,
            statuses,
            people,
        }
    }

    fn chip_of(changes: &[Change]) -> Option<String> {
        let statuses = map(&[("10002", "In Progress"), ("10003", "In Review")]);
        let people = map(&[(MARTA, "Marta Klein")]);
        chip(&facts(changes), &names(Some(ME), &statuses, &people))
    }

    fn sentence_of(changes: &[Change]) -> Option<String> {
        let statuses = map(&[("10003", "In Review")]);
        let people = map(&[(MARTA, "Marta Klein")]);
        sentence(&facts(changes), &names(Some(ME), &statuses, &people))
    }

    #[test]
    fn the_chip_names_the_most_important_change() {
        let cases: Vec<(Vec<Change>, Option<&str>)> = vec![
            (vec![], None),
            (vec![change("created", None, Some("NW-1"))], Some("New")),
            (
                vec![change("comment_added", Some("4"), Some("5"))],
                Some("1 new comment"),
            ),
            (
                vec![
                    change("comment_added", Some("1"), Some("2")),
                    change("comment_added", Some("2"), Some("4")),
                ],
                Some("3 new comments"),
            ),
            (
                vec![change("status", Some("10002"), Some("10003"))],
                Some("Moved to In Review"),
            ),
            (
                vec![change("status", Some("10002"), Some("10009"))],
                Some("Moved to 10009"),
            ),
            (
                vec![
                    change("status", Some("10002"), Some("10003")),
                    change("status", Some("10003"), Some("10002")),
                ],
                Some("Moved to In Progress"),
            ),
            (
                vec![change("priority", Some("Medium"), Some("High"))],
                Some("Priority raised"),
            ),
            (
                vec![change("priority", Some("High"), Some("Low"))],
                Some("Priority lowered"),
            ),
            (
                vec![change("priority", None, Some("High"))],
                Some("Priority raised"),
            ),
            (
                vec![change("priority", Some("High"), None)],
                Some("Priority lowered"),
            ),
            (
                vec![
                    change("priority", Some("Medium"), Some("High")),
                    change("priority", Some("High"), Some("Medium")),
                ],
                None,
            ),
            (
                vec![change("assignee", None, Some(ME))],
                Some("Assigned to you"),
            ),
            (
                vec![change("assignee", Some(ME), Some(MARTA))],
                Some("Reassigned"),
            ),
            (vec![change("assignee", Some(ME), None)], Some("Unassigned")),
            (
                vec![change("due_date", Some("2026-10-30"), Some("2026-11-02"))],
                Some("Due date changed"),
            ),
            (
                vec![
                    change("status", Some("10002"), Some("10003")),
                    change("comment_added", Some("1"), Some("2")),
                ],
                Some("1 new comment +1"),
            ),
            (
                vec![
                    change("due_date", None, Some("2026-11-02")),
                    change("assignee", None, Some(MARTA)),
                    change("priority", Some("Low"), Some("Highest")),
                ],
                Some("Priority raised +2"),
            ),
            (
                vec![
                    change("created", None, Some("NW-1")),
                    change("comment_added", Some("0"), Some("1")),
                ],
                Some("New +1"),
            ),
            (vec![change("gone", None, None)], None),
        ];
        for (changes, expected) in cases {
            assert_eq!(chip_of(&changes).as_deref(), expected, "{changes:?}");
        }
    }

    #[test]
    fn a_chip_without_names_falls_back_to_ids() {
        let none = HashMap::new();

        let found = chip(
            &facts(&[change("status", None, Some("10003"))]),
            &names(None, &none, &none),
        );

        assert_eq!(found.as_deref(), Some("Moved to 10003"));
    }

    #[test]
    fn nobody_is_me_when_the_account_is_unknown() {
        let none = HashMap::new();

        let found = chip(
            &facts(&[change("assignee", None, Some(ME))]),
            &names(None, &none, &none),
        );

        assert_eq!(found.as_deref(), Some("Reassigned"));
    }

    #[test]
    fn facts_keep_each_kind_once_in_priority_order() {
        let found = facts(&[
            change("due_date", None, Some("2026-11-02")),
            change("assignee", None, Some(MARTA)),
            change("assignee", Some(MARTA), Some(ME)),
            change("priority", Some("Medium"), Some("Highest")),
            change("status", Some("10002"), Some("10003")),
            change("comment_added", Some("1"), Some("3")),
            change("created", None, Some("NW-1")),
        ]);

        assert_eq!(
            found,
            [
                Fact::New,
                Fact::Comments(2),
                Fact::Moved("10003".into()),
                Fact::PriorityRaised,
                Fact::Assigned(Some(ME.into())),
                Fact::Due,
            ]
        );
    }

    #[test]
    fn the_sentence_lists_every_change_as_a_clause() {
        let cases: Vec<(Vec<Change>, Option<&str>)> = vec![
            (vec![], None),
            (
                vec![
                    change("comment_added", Some("1"), Some("3")),
                    change("status", Some("10002"), Some("10003")),
                ],
                Some("2 new comments, moved to In Review."),
            ),
            (
                vec![change("comment_added", Some("1"), Some("2"))],
                Some("1 new comment."),
            ),
            (
                vec![change("created", None, Some("NW-1"))],
                Some("created."),
            ),
            (
                vec![change("assignee", Some(ME), Some(MARTA))],
                Some("reassigned to Marta Klein."),
            ),
            (
                vec![change("assignee", Some(ME), Some("account-unknown"))],
                Some("reassigned to account-unknown."),
            ),
            (
                vec![change("assignee", None, Some(ME))],
                Some("assigned to you."),
            ),
            (
                vec![change("assignee", Some(ME), None)],
                Some("unassigned."),
            ),
            (
                vec![
                    change("priority", Some("Low"), Some("High")),
                    change("due_date", None, Some("2026-11-02")),
                ],
                Some("priority raised, due date changed."),
            ),
            (
                vec![change("priority", Some("High"), Some("Low"))],
                Some("priority lowered."),
            ),
        ];
        for (changes, expected) in cases {
            assert_eq!(sentence_of(&changes).as_deref(), expected, "{changes:?}");
        }
    }
}
