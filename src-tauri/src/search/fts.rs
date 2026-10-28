//! Typed text as an FTS5 query and as an issue key.

/// More words than this are ignored.
const MAX_TERMS: usize = 12;
/// Longer words are cut.
const MAX_TERM_CHARS: usize = 64;

/// Every word as a quoted prefix query, all required: `check total` → `"check"* "total"*`. Quoting keeps FTS5
/// syntax (`NEAR`, `*`, `-`, quotes) literal; words without a letter or digit are dropped. None when nothing
/// searchable is left.
pub fn match_expression(text: &str) -> Option<String> {
    let terms: Vec<String> = text
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .take(MAX_TERMS)
        .map(|word| {
            let word: String = word.chars().take(MAX_TERM_CHARS).collect();
            format!("\"{}\"*", word.replace('"', "\"\""))
        })
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

/// What the text says about a key: a whole key (`nw-1042` → `NW-1042`) or a bare number (`1042`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyHint {
    Key(String),
    Number(String),
}

pub fn key_hint(text: &str) -> Option<KeyHint> {
    let text = text.trim();
    if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit()) {
        return Some(KeyHint::Number(text.to_string()));
    }
    let (project, number) = text.split_once('-')?;
    let project_ok = project
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && project
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    let number_ok = !number.is_empty() && number.chars().all(|c| c.is_ascii_digit());
    (project_ok && number_ok).then(|| KeyHint::Key(text.to_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbered_words(count: usize) -> String {
        (1..=count)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn words_become_quoted_prefix_terms_joined_by_spaces() {
        for (input, expected) in [
            ("checkout", r#""checkout"*"#),
            ("check total", r#""check"* "total"*"#),
            ("  check \t total\n", r#""check"* "total"*"#),
            ("Zählung", r#""Zählung"*"#),
            ("NW-1042", r#""NW-1042"*"#),
        ] {
            assert_eq!(
                match_expression(input).as_deref(),
                Some(expected),
                "{input:?}"
            );
        }
    }

    #[test]
    fn quotes_inside_a_word_are_doubled() {
        for (input, expected) in [
            (r#"say "hi""#, r#""say"* """hi"""*"#),
            (r#"a"b"#, r#""a""b"*"#),
        ] {
            assert_eq!(
                match_expression(input).as_deref(),
                Some(expected),
                "{input}"
            );
        }
    }

    #[test]
    fn fts_syntax_words_and_characters_stay_literal() {
        for (input, expected) in [
            ("NEAR", r#""NEAR"*"#),
            ("AND OR NOT", r#""AND"* "OR"* "NOT"*"#),
            ("NEAR(a", r#""NEAR(a"*"#),
            ("-x", r#""-x"*"#),
            ("a*", r#""a*"*"#),
            ("(a)", r#""(a)"*"#),
            ("title:foo", r#""title:foo"*"#),
        ] {
            assert_eq!(
                match_expression(input).as_deref(),
                Some(expected),
                "{input}"
            );
        }
    }

    #[test]
    fn words_without_a_letter_or_digit_are_dropped() {
        for (input, expected) in [
            ("check * total", Some(r#""check"* "total"*"#)),
            ("- check ( ) \"", Some(r#""check"*"#)),
            ("*", None),
            ("\"", None),
            ("(", None),
            ("- * \" ( ) ,", None),
        ] {
            assert_eq!(match_expression(input).as_deref(), expected, "{input}");
        }
    }

    #[test]
    fn empty_or_blank_text_has_no_expression() {
        for input in ["", " ", " \t\n "] {
            assert_eq!(match_expression(input), None, "{input:?}");
        }
    }

    #[test]
    fn at_most_twelve_words_are_searched() {
        let expected = numbered_words(12)
            .split(' ')
            .map(|word| format!("\"{word}\"*"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            match_expression(&numbered_words(13)).as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(
            match_expression(&numbered_words(12)).as_deref(),
            Some(expected.as_str())
        );
    }

    #[test]
    fn dropped_words_do_not_count_toward_the_word_limit() {
        let text = format!("- * {}", numbered_words(12));
        let expression = match_expression(&text).unwrap();
        assert_eq!(expression.split(' ').count(), 12);
        assert!(expression.starts_with(r#""w1"*"#));
        assert!(expression.ends_with(r#""w12"*"#));
    }

    #[test]
    fn a_word_over_64_characters_is_cut() {
        for letter in ["a", "é"] {
            let word = letter.repeat(70);
            assert_eq!(
                match_expression(&word),
                Some(format!("\"{}\"*", letter.repeat(64))),
                "{letter}"
            );
        }
        let exact = "b".repeat(64);
        assert_eq!(match_expression(&exact), Some(format!("\"{exact}\"*")));
    }

    #[test]
    fn a_cut_word_does_not_leave_half_a_doubled_quote() {
        let word = format!("{}\"", "a".repeat(64));
        assert_eq!(
            match_expression(&word),
            Some(format!("\"{}\"*", "a".repeat(64)))
        );
    }

    #[test]
    fn a_whole_key_is_upper_cased() {
        for (input, expected) in [
            ("nw-1042", "NW-1042"),
            ("NW-1042", "NW-1042"),
            ("  nw-7 ", "NW-7"),
            ("a1_b-3", "A1_B-3"),
        ] {
            assert_eq!(
                key_hint(input),
                Some(KeyHint::Key(expected.to_string())),
                "{input:?}"
            );
        }
    }

    #[test]
    fn digits_alone_are_a_number() {
        for (input, expected) in [("1042", "1042"), (" 7 ", "7"), ("007", "007")] {
            assert_eq!(
                key_hint(input),
                Some(KeyHint::Number(expected.to_string())),
                "{input:?}"
            );
        }
    }

    #[test]
    fn text_that_is_not_a_key_or_number_has_no_hint() {
        for input in [
            "", " ", "NW-", "-12", "N W-1", "1a2", "nw-12a", "nw-1-2", "1nw-5", "checkout", "-",
        ] {
            assert_eq!(key_hint(input), None, "{input:?}");
        }
    }
}
