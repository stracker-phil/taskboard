//! `@Full Name` of someone who may be mentioned, in typed text.

use super::Mentionable;

pub(super) enum Part<'t, 'p> {
    Text(&'t str),
    Mention(&'p Mentionable),
}

/// The text as plain runs and mentions. An `@` counts at the start of a word only (not in `name@example.com`),
/// and the longest name that ends at a word boundary wins.
pub(super) fn split<'t, 'p>(text: &'t str, people: &'p [Mentionable]) -> Vec<Part<'t, 'p>> {
    let mut parts = Vec::new();
    let (mut plain_from, mut search_from) = (0, 0);
    while let Some(offset) = text[search_from..].find('@') {
        let at = search_from + offset;
        let found = if starts_word(&text[..at]) {
            person_named_at(people, &text[at + 1..])
        } else {
            None
        };
        match found {
            Some(person) => {
                if plain_from < at {
                    parts.push(Part::Text(&text[plain_from..at]));
                }
                parts.push(Part::Mention(person));
                search_from = at + 1 + person.name.len();
                plain_from = search_from;
            }
            None => search_from = at + 1,
        }
    }
    if plain_from < text.len() {
        parts.push(Part::Text(&text[plain_from..]));
    }
    parts
}

/// The person whose name the text after an `@` starts with.
fn person_named_at<'p>(people: &'p [Mentionable], after_at: &str) -> Option<&'p Mentionable> {
    people
        .iter()
        .filter(|p| {
            !p.name.is_empty()
                && after_at.starts_with(&p.name)
                && ends_word(&after_at[p.name.len()..])
        })
        .max_by_key(|p| p.name.len())
}

fn starts_word(before: &str) -> bool {
    before
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric())
}

fn ends_word(after: &str) -> bool {
    after.chars().next().is_none_or(|c| !c.is_alphanumeric())
}
