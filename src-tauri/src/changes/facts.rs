//! Unseen changes in words: the Since-yesterday sentence and the inbox's lines.

use std::collections::HashMap;

use super::Change;
use crate::views::rows;

/// What a run of changes amounts to; each kind once, with its latest value.
#[derive(Debug, Clone, PartialEq)]
pub enum Fact {
    /// New comments mention accounts; only mentions of "you" count.
    Mentioned(Vec<Mention>),
    /// Accounts added to people fields; only "you" counts.
    AddedAs(Vec<PeopleChange>),
    New,
    Comments(i64),
    /// To this status id.
    Moved(String),
    PriorityRaised,
    PriorityLowered,
    /// To this account id; none when unassigned.
    Assigned(Option<String>),
    Due,
    /// Updated after the owner last looked, while the cache didn't hold the issue (`issue_change` `updated`).
    Updated,
}

/// A new comment's mention of an account.
#[derive(Debug, Clone, PartialEq)]
pub struct Mention {
    pub comment: String,
    pub account: String,
}

/// An account added to a people field (`issue_change` `people`).
#[derive(Debug, Clone, PartialEq)]
pub struct PeopleChange {
    pub field: String,
    pub account: String,
}

/// The ids in the facts resolved to names.
pub struct Names<'a> {
    pub me: Option<&'a str>,
    pub statuses: &'a HashMap<String, String>,
    pub people: &'a HashMap<String, String>,
    /// Custom field ids to their names.
    pub fields: &'a HashMap<String, String>,
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
    let mentioned: Vec<Mention> = of("mention")
        .filter_map(|c| {
            Some(Mention {
                comment: c.old_value.clone()?,
                account: c.new_value.clone()?,
            })
        })
        .collect();
    if !mentioned.is_empty() {
        facts.push(Fact::Mentioned(mentioned));
    }
    let added: Vec<PeopleChange> = of("people")
        .filter_map(|c| {
            Some(PeopleChange {
                field: c.old_value.clone()?,
                account: c.new_value.clone()?,
            })
        })
        .collect();
    if !added.is_empty() {
        facts.push(Fact::AddedAs(added));
    }
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
    if of("updated").next().is_some() {
        facts.push(Fact::Updated);
    }
    facts
}

fn status<'a>(names: &'a Names, id: &'a str) -> &'a str {
    names.statuses.get(id).map_or(id, String::as_str)
}

/// "Developer", "Developer and Reviewer": each field once, unknown ids as they are.
fn field_names(added: &[PeopleChange], names: &Names) -> String {
    let mut fields: Vec<&str> = Vec::new();
    for change in added {
        let name = names
            .fields
            .get(&change.field)
            .map_or(change.field.as_str(), String::as_str);
        if !fields.contains(&name) {
            fields.push(name);
        }
    }
    match fields.split_last() {
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

fn comments(n: i64) -> String {
    if n == 1 {
        "1 new comment".into()
    } else {
        format!("{n} new comments")
    }
}

/// The facts as the owner reads them: mentions of others go, and comments that mention the owner count as the
/// mention, not again as new comments.
fn for_me(facts: &[Fact], names: &Names) -> Vec<Fact> {
    let mine: std::collections::BTreeSet<&str> = facts
        .iter()
        .flat_map(|fact| match fact {
            Fact::Mentioned(mentions) => mentions.as_slice(),
            _ => &[],
        })
        .filter(|m| names.me == Some(m.account.as_str()))
        .map(|m| m.comment.as_str())
        .collect();
    facts
        .iter()
        .filter_map(|fact| match fact {
            Fact::Mentioned(_) if mine.is_empty() => None,
            Fact::AddedAs(added) => {
                let as_me: Vec<PeopleChange> = added
                    .iter()
                    .filter(|a| names.me == Some(a.account.as_str()))
                    .cloned()
                    .collect();
                (!as_me.is_empty()).then_some(Fact::AddedAs(as_me))
            }
            Fact::Comments(n) => {
                let others = n - mine.len() as i64;
                (others > 0).then_some(Fact::Comments(others))
            }
            other => Some(other.clone()),
        })
        .collect()
}

/// Every fact as a clause of the banner's sentence ("2 new comments, moved to Review.").
pub fn sentence(facts: &[Fact], names: &Names) -> Option<String> {
    let clauses: Vec<String> = for_me(facts, names)
        .iter()
        .map(|fact| match fact {
            Fact::Mentioned(_) => "mentioned you".into(),
            Fact::AddedAs(added) => format!("added you as {}", field_names(added, names)),
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
            Fact::Updated => "updated".into(),
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
        static NO_FIELDS: std::sync::LazyLock<HashMap<String, String>> =
            std::sync::LazyLock::new(HashMap::new);
        Names {
            me,
            statuses,
            people,
            fields: &NO_FIELDS,
        }
    }

    fn sentence_of(changes: &[Change]) -> Option<String> {
        let statuses = map(&[("10003", "In Review")]);
        let people = map(&[(MARTA, "Marta Klein")]);
        sentence(&facts(changes), &names(Some(ME), &statuses, &people))
    }

    #[test]
    fn the_sentence_orders_clauses_and_merges_runs_of_changes() {
        let cases: Vec<(Vec<Change>, Option<&str>)> = vec![
            (
                vec![
                    change("comment_added", Some("1"), Some("2")),
                    change("comment_added", Some("2"), Some("4")),
                ],
                Some("3 new comments."),
            ),
            (
                vec![change("status", Some("10002"), Some("10009"))],
                Some("moved to 10009."),
            ),
            (
                vec![
                    change("status", Some("10002"), Some("10003")),
                    change("status", Some("10003"), Some("10002")),
                ],
                Some("moved to 10002."),
            ),
            (
                vec![change("priority", None, Some("High"))],
                Some("priority raised."),
            ),
            (
                vec![change("priority", Some("High"), None)],
                Some("priority lowered."),
            ),
            (
                vec![
                    change("priority", Some("Medium"), Some("High")),
                    change("priority", Some("High"), Some("Medium")),
                ],
                None,
            ),
            (
                vec![
                    change("status", Some("10002"), Some("10003")),
                    change("comment_added", Some("1"), Some("2")),
                ],
                Some("1 new comment, moved to In Review."),
            ),
            (
                vec![
                    change("due_date", None, Some("2026-11-02")),
                    change("assignee", None, Some(MARTA)),
                    change("priority", Some("Low"), Some("Highest")),
                ],
                Some("priority raised, reassigned to Marta Klein, due date changed."),
            ),
            (
                vec![
                    change("created", None, Some("NW-1")),
                    change("comment_added", Some("0"), Some("1")),
                ],
                Some("created, 1 new comment."),
            ),
            (vec![change("gone", None, None)], None),
        ];
        for (changes, expected) in cases {
            assert_eq!(sentence_of(&changes).as_deref(), expected, "{changes:?}");
        }
    }

    #[test]
    fn a_sentence_without_names_falls_back_to_ids() {
        let none = HashMap::new();

        let found = sentence(
            &facts(&[change("status", None, Some("10003"))]),
            &names(None, &none, &none),
        );

        assert_eq!(found.as_deref(), Some("moved to 10003."));
    }

    #[test]
    fn nobody_is_me_when_the_account_is_unknown() {
        let none = HashMap::new();

        let found = sentence(
            &facts(&[change("assignee", None, Some(ME))]),
            &names(None, &none, &none),
        );

        assert_eq!(found.as_deref(), Some("reassigned to account-me."));
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

    fn mention(comment: &str, account: &str) -> Change {
        change("mention", Some(comment), Some(account))
    }

    #[test]
    fn mentions_are_one_fact_that_ranks_first() {
        let found = facts(&[
            change("comment_added", Some("1"), Some("3")),
            mention("90001", MARTA),
            change("created", None, Some("NW-1")),
            mention("90002", ME),
        ]);

        assert_eq!(
            found,
            [
                Fact::Mentioned(vec![
                    Mention {
                        comment: "90001".into(),
                        account: MARTA.into()
                    },
                    Mention {
                        comment: "90002".into(),
                        account: ME.into()
                    },
                ]),
                Fact::New,
                Fact::Comments(2),
            ]
        );
    }

    #[test]
    fn a_mention_comment_counts_as_the_mention_not_again_as_a_new_comment() {
        let cases: Vec<(Vec<Change>, Option<&str>)> = vec![
            (vec![mention("90001", ME)], Some("mentioned you.")),
            (
                vec![mention("90001", MARTA), mention("90002", ME)],
                Some("mentioned you."),
            ),
            (
                vec![
                    mention("90001", ME),
                    mention("90002", ME),
                    change("comment_added", Some("4"), Some("6")),
                ],
                Some("mentioned you."),
            ),
            (
                vec![
                    mention("90001", ME),
                    change("comment_added", Some("4"), Some("5")),
                    change("status", Some("10002"), Some("10003")),
                ],
                Some("mentioned you, moved to In Review."),
            ),
        ];
        for (changes, expected) in cases {
            assert_eq!(sentence_of(&changes).as_deref(), expected, "{changes:?}");
        }
    }

    #[test]
    fn the_sentence_says_mentioned_you_first_and_skips_mentions_of_others() {
        let cases: Vec<(Vec<Change>, Option<&str>)> = vec![
            (
                vec![
                    change("comment_added", Some("4"), Some("5")),
                    mention("90001", ME),
                ],
                Some("mentioned you."),
            ),
            (
                vec![
                    change("comment_added", Some("4"), Some("6")),
                    mention("90001", ME),
                ],
                Some("mentioned you, 1 new comment."),
            ),
            (
                vec![
                    change("comment_added", Some("4"), Some("5")),
                    mention("90001", MARTA),
                ],
                Some("1 new comment."),
            ),
            (vec![mention("90001", MARTA)], None),
        ];
        for (changes, expected) in cases {
            assert_eq!(sentence_of(&changes).as_deref(), expected, "{changes:?}");
        }
    }

    #[test]
    fn without_a_connected_account_mentions_never_count() {
        let none = HashMap::new();
        let changes = [
            mention("90001", ME),
            change("comment_added", Some("4"), Some("5")),
        ];
        let facts = facts(&changes);

        assert_eq!(
            sentence(&facts, &names(None, &none, &none)).as_deref(),
            Some("1 new comment.")
        );
    }

    const DEVELOPER: &str = "customfield_10042";
    const REVIEWER: &str = "customfield_10043";
    const TESTER: &str = "customfield_10044";

    fn people_change(field: &str, account: &str) -> Change {
        change("people", Some(field), Some(account))
    }

    fn sentence_with_fields(changes: &[Change]) -> Option<String> {
        let none = HashMap::new();
        let fields = map(&[
            (DEVELOPER, "Developer"),
            (REVIEWER, "Reviewer"),
            (TESTER, "Tester"),
        ]);
        sentence(
            &facts(changes),
            &Names {
                me: Some(ME),
                statuses: &none,
                people: &none,
                fields: &fields,
            },
        )
    }

    #[test]
    fn being_added_to_people_fields_is_one_fact_that_ranks_after_mentions() {
        let found = facts(&[
            change("created", None, Some("NW-1")),
            people_change(DEVELOPER, ME),
            people_change(REVIEWER, MARTA),
            mention("90001", ME),
        ]);

        assert_eq!(
            found,
            [
                Fact::Mentioned(vec![Mention {
                    comment: "90001".into(),
                    account: ME.into()
                }]),
                Fact::AddedAs(vec![
                    PeopleChange {
                        field: DEVELOPER.into(),
                        account: ME.into()
                    },
                    PeopleChange {
                        field: REVIEWER.into(),
                        account: MARTA.into()
                    },
                ]),
                Fact::New,
            ]
        );
    }

    #[test]
    fn the_sentence_says_added_you_as_the_field_names() {
        let cases: Vec<(Vec<Change>, Option<&str>)> = vec![
            (
                vec![people_change(DEVELOPER, ME)],
                Some("added you as Developer."),
            ),
            (
                vec![people_change(DEVELOPER, ME), people_change(REVIEWER, ME)],
                Some("added you as Developer and Reviewer."),
            ),
            (
                vec![
                    people_change(DEVELOPER, ME),
                    people_change(REVIEWER, ME),
                    people_change(TESTER, ME),
                ],
                Some("added you as Developer, Reviewer and Tester."),
            ),
            (
                vec![people_change(DEVELOPER, ME), people_change(DEVELOPER, ME)],
                Some("added you as Developer."),
            ),
            (
                vec![people_change("customfield_10099", ME)],
                Some("added you as customfield_10099."),
            ),
            (
                vec![
                    people_change(DEVELOPER, ME),
                    change("comment_added", Some("1"), Some("2")),
                ],
                Some("added you as Developer, 1 new comment."),
            ),
        ];
        for (changes, expected) in cases {
            assert_eq!(
                sentence_with_fields(&changes).as_deref(),
                expected,
                "{changes:?}"
            );
        }
    }

    #[test]
    fn rows_for_another_account_do_not_count_in_the_sentence() {
        let cases: Vec<(Vec<Change>, Option<&str>)> = vec![
            (vec![people_change(DEVELOPER, MARTA)], None),
            (
                vec![people_change(DEVELOPER, MARTA), people_change(REVIEWER, ME)],
                Some("added you as Reviewer."),
            ),
        ];
        for (changes, expected) in cases {
            assert_eq!(
                sentence_with_fields(&changes).as_deref(),
                expected,
                "{changes:?}"
            );
        }
    }

    #[test]
    fn without_a_connected_account_people_rows_never_count() {
        let none = HashMap::new();
        let facts = facts(&[people_change(DEVELOPER, ME)]);

        assert_eq!(sentence(&facts, &names(None, &none, &none)), None);
    }

    #[test]
    fn an_updated_record_is_the_updated_fact_with_a_clause() {
        let changes = [change("updated", None, None)];

        assert_eq!(facts(&changes), [Fact::Updated]);
        assert_eq!(sentence_of(&changes).as_deref(), Some("updated."));
    }

    #[test]
    fn updated_comes_last_in_the_facts_and_the_sentence() {
        let changes = [
            change("updated", None, None),
            change("status", Some("10002"), Some("10003")),
            change("due_date", None, Some("2026-11-02")),
        ];

        assert_eq!(
            facts(&changes),
            [Fact::Moved("10003".into()), Fact::Due, Fact::Updated]
        );
        assert_eq!(
            sentence_of(&changes).as_deref(),
            Some("moved to In Review, due date changed, updated.")
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
