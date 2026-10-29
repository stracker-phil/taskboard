//! Validated values, the only way to name a Jira object or pass text in a request. Values that go into a
//! URL path are limited to characters that can't change the URL's shape.

use crate::error::{AppError, AppResult};

/// Jira's limit for `issue/bulkfetch`.
pub const BULK_FETCH_MAX: usize = 100;
const MAX_PROJECT_KEY: usize = 255;
const MAX_ISSUE_ID: usize = 20;
const MAX_JQL: usize = 10_000;
const MAX_FIELD: usize = 100;
const MAX_FIELDS: usize = 500;
const MAX_PAGE_TOKEN: usize = 4096;
/// The `expand` values a search may ask for.
const EXPANDS: &[&str] = &["renderedFields"];

/// An uppercase ASCII letter, then uppercase letters, digits or `_` (Jira's rule for project keys).
pub(super) fn is_project_key(text: &str) -> bool {
    let mut chars = text.chars();
    text.len() <= MAX_PROJECT_KEY
        && chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

pub(super) fn is_issue_id(text: &str) -> bool {
    !text.is_empty() && text.len() <= MAX_ISSUE_ID && text.bytes().all(|b| b.is_ascii_digit())
}

pub(super) fn is_issue_key(text: &str) -> bool {
    text.rsplit_once('-')
        .is_some_and(|(project, number)| is_project_key(project) && is_issue_id(number))
}

/// A project key, as in `project/{key}/versions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectKey(String);

impl ProjectKey {
    pub fn parse(text: &str) -> AppResult<Self> {
        if is_project_key(text) {
            Ok(Self(text.into()))
        } else {
            Err(refused("project key", text))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An issue's numeric id or its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueRef(String);

impl IssueRef {
    pub fn parse(text: &str) -> AppResult<Self> {
        if is_issue_id(text) || is_issue_key(text) {
            Ok(Self(text.into()))
        } else {
            Err(refused("issue id or key", text))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An attachment's numeric id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentId(String);

impl AttachmentId {
    pub fn parse(text: &str) -> AppResult<Self> {
        if is_issue_id(text) {
            Ok(Self(text.into()))
        } else {
            Err(refused("attachment id", text))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 1 to `BULK_FETCH_MAX` issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueRefs(Vec<IssueRef>);

impl IssueRefs {
    pub fn parse(items: &[String]) -> AppResult<Self> {
        if items.is_empty() || items.len() > BULK_FETCH_MAX {
            return Err(AppError::InvalidInput(format!(
                "Taskboard fetches 1 to {BULK_FETCH_MAX} issues at a time, not {}",
                items.len()
            )));
        }
        items
            .iter()
            .map(|item| IssueRef::parse(item))
            .collect::<AppResult<_>>()
            .map(Self)
    }

    pub fn as_strs(&self) -> Vec<&str> {
        self.0.iter().map(IssueRef::as_str).collect()
    }
}

/// A JQL query; it only ever travels in a JSON body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jql(String);

impl Jql {
    pub fn parse(text: &str) -> AppResult<Self> {
        if text.trim().is_empty() || text.chars().count() > MAX_JQL || text.contains('\0') {
            Err(refused("query", text))
        } else {
            Ok(Self(text.into()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Field ids to fetch: always an explicit list, never `*all` or `*navigable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldIds(Vec<String>);

impl FieldIds {
    pub fn parse(items: &[String]) -> AppResult<Self> {
        if items.is_empty() || items.len() > MAX_FIELDS {
            return Err(AppError::InvalidInput(format!(
                "Taskboard asks for 1 to {MAX_FIELDS} fields, not {}",
                items.len()
            )));
        }
        for item in items {
            let valid = !item.is_empty()
                && item.len() <= MAX_FIELD
                && item
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.');
            if !valid {
                return Err(refused("field id", item));
            }
        }
        Ok(Self(items.to_vec()))
    }

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

/// A search's `expand`: one of `EXPANDS`, comma-separated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expand(String);

impl Expand {
    pub fn parse(text: &str) -> AppResult<Self> {
        if !text.is_empty() && text.split(',').all(|part| EXPANDS.contains(&part)) {
            Ok(Self(text.into()))
        } else {
            Err(refused("expand", text))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The opaque `nextPageToken` from Jira's last search page; it only travels in a JSON body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageToken(String);

impl PageToken {
    pub fn parse(text: &str) -> AppResult<Self> {
        if text.is_empty() || text.len() > MAX_PAGE_TOKEN || text.chars().any(char::is_control) {
            Err(refused("page token", text))
        } else {
            Ok(Self(text.into()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The value is cut short: it may be long or carry control characters.
fn refused(what: &str, text: &str) -> AppError {
    let shown: String = text
        .chars()
        .take(40)
        .collect::<String>()
        .escape_debug()
        .to_string();
    AppError::InvalidInput(format!(
        "Taskboard won't send the {what} \"{shown}\" to Jira"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    fn is_invalid<T>(result: &AppResult<T>) -> bool {
        matches!(result, Err(AppError::InvalidInput(_)))
    }

    fn message<T: std::fmt::Debug>(result: AppResult<T>) -> String {
        match result {
            Err(AppError::InvalidInput(message)) => message,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// Texts that must never name a project or an issue.
    fn escaping_texts() -> Vec<String> {
        let mut texts = strings(&[
            "../X",
            "X/1",
            "X?a=b",
            "X#f",
            "%2e%2e",
            "%2E%2E",
            "N%57",
            " NW",
            "NW ",
            "\tNW",
            "NW\n",
            "N W",
            "N\0W",
            "NW\0",
            "\u{202E}NW",
            "NW\u{202E}",
            "N\u{200B}W",
            "nw",
            "Nw",
            "",
            "ＮＷ",
            "NW/",
            "/",
            "..",
            ".",
            "NW.",
            "NW&x=1",
            "NW=",
            "NW\\",
            "NW:",
            "1NW",
            "_NW",
            "é",
        ]);
        texts.extend(
            (0u8..=31)
                .chain([127])
                .map(|byte| format!("N{}W", char::from(byte))),
        );
        texts.push("A".repeat(256));
        texts.push("A".repeat(5000));
        texts
    }

    #[test]
    fn project_keys_are_uppercase_ascii_names() {
        let longest = "A".repeat(255);
        for key in ["NW", "A1_B", "LUM", "N", longest.as_str()] {
            assert_eq!(ProjectKey::parse(key).unwrap().as_str(), key);
        }
    }

    #[test]
    fn a_project_key_that_could_leave_its_path_segment_is_refused() {
        for text in escaping_texts().iter().map(String::as_str).chain(["NW-1"]) {
            assert!(is_invalid(&ProjectKey::parse(text)), "{text:?}");
        }
    }

    #[test]
    fn issues_are_named_by_numeric_id_or_key() {
        let longest_id = "1".repeat(20);
        for issue in ["10001", "NW-1042", "A1_B-7", "LUM-1", longest_id.as_str()] {
            assert_eq!(IssueRef::parse(issue).unwrap().as_str(), issue);
        }
    }

    #[test]
    fn an_issue_reference_that_could_leave_its_path_segment_is_refused() {
        let long_id = "1".repeat(21);
        let long_key = format!("NW-{long_id}");
        let extra = [
            "١٢٣",
            "NW-١٢٣",
            "NW",
            "NW-",
            "-1",
            "nw-1",
            "NW-1-2",
            "NW--1",
            "NW-1 ",
            "NW-1/comment",
            "1/2",
            "1.5",
            "+1",
            "NW_1",
            "1e5",
            "../1",
            long_id.as_str(),
            long_key.as_str(),
        ];
        for text in escaping_texts().iter().map(String::as_str).chain(extra) {
            assert!(is_invalid(&IssueRef::parse(text)), "{text:?}");
        }
    }

    #[test]
    fn a_list_of_issues_holds_one_to_a_hundred_valid_references() {
        let hundred: Vec<String> = (1..=100).map(|n| format!("NW-{n}")).collect();
        let refs = IssueRefs::parse(&hundred).unwrap();
        assert_eq!(
            refs.as_strs(),
            hundred.iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert_eq!(
            IssueRefs::parse(&strings(&["10001", "LUM-7"]))
                .unwrap()
                .as_strs(),
            ["10001", "LUM-7"]
        );
    }

    #[test]
    fn an_empty_oversized_or_partly_invalid_issue_list_is_refused() {
        let hundred_and_one: Vec<String> = (1..=101).map(|n| format!("NW-{n}")).collect();
        let mut one_bad: Vec<String> = (1..=5).map(|n| format!("NW-{n}")).collect();
        one_bad.push("../1".into());
        for (label, items) in [
            ("empty", Vec::new()),
            ("101 items", hundred_and_one),
            ("one invalid item", one_bad),
            ("empty item", strings(&["NW-1", ""])),
        ] {
            assert!(is_invalid(&IssueRefs::parse(&items)), "{label}");
        }
    }

    #[test]
    fn a_query_may_hold_quotes_symbols_and_line_breaks_up_to_10000_characters() {
        let longest = "a".repeat(10_000);
        let wide = "é".repeat(10_000);
        for text in [
            "x",
            r#"project = "North Wind" AND text ~ "a&b?c#d""#,
            "project = NW\nORDER BY created",
            "a%2Fb",
            longest.as_str(),
            wide.as_str(),
        ] {
            assert_eq!(Jql::parse(text).unwrap().as_str(), text);
        }
    }

    #[test]
    fn an_empty_blank_nul_or_oversized_query_is_refused() {
        let too_long = "a".repeat(10_001);
        for text in ["", " ", "\n\t ", "\0", "a\0b", too_long.as_str()] {
            assert!(is_invalid(&Jql::parse(text)), "{text:?}");
        }
    }

    #[test]
    fn field_ids_are_an_explicit_list_of_plain_names() {
        let fields = FieldIds::parse(&strings(&["summary", "customfield_10016"])).unwrap();
        assert_eq!(fields.as_slice(), ["summary", "customfield_10016"]);
    }

    #[test]
    fn field_lists_that_widen_or_break_the_request_are_refused() {
        let too_many: Vec<String> = (0..501).map(|n| format!("customfield_{n}")).collect();
        let too_long = vec!["a".repeat(101)];
        let cases: Vec<(&str, Vec<String>)> = vec![
            ("empty list", Vec::new()),
            ("empty item", strings(&["summary", ""])),
            ("*all", strings(&["*all"])),
            ("*navigable", strings(&["*navigable"])),
            ("exclusion", strings(&["-summary"])),
            ("comma", strings(&["a,b"])),
            ("space", strings(&["sum mary"])),
            ("leading space", strings(&[" summary"])),
            ("newline", strings(&["summary\n"])),
            ("non-ASCII", strings(&["sümmary"])),
            ("501 fields", too_many),
            ("long item", too_long),
        ];
        for (label, items) in cases {
            assert!(is_invalid(&FieldIds::parse(&items)), "{label}");
        }
    }

    #[test]
    fn only_rendered_fields_can_be_expanded() {
        for text in ["renderedFields", "renderedFields,renderedFields"] {
            assert_eq!(Expand::parse(text).unwrap().as_str(), text);
        }
        for text in [
            "changelog",
            "",
            "renderedFields,changelog",
            "renderedFields,",
            ",renderedFields",
            "renderedfields",
            " renderedFields",
            "names",
        ] {
            assert!(is_invalid(&Expand::parse(text)), "{text:?}");
        }
    }

    #[test]
    fn a_page_token_is_opaque_text_up_to_4096_bytes_without_control_characters() {
        let longest = "t".repeat(4096);
        for token in ["abc123==", "a b+c/d", longest.as_str()] {
            assert_eq!(PageToken::parse(token).unwrap().as_str(), token);
        }
        let too_long = "t".repeat(4097);
        for token in [
            "",
            "a\nb",
            "a\0b",
            "a\u{1b}b",
            "a\u{7f}b",
            too_long.as_str(),
        ] {
            assert!(is_invalid(&PageToken::parse(token)), "{token:?}");
        }
    }

    #[test]
    fn a_refusal_shows_at_most_40_characters_of_the_value() {
        let long = "a".repeat(300);
        for text in [
            message(ProjectKey::parse(&long)),
            message(IssueRef::parse(&long)),
            message(Jql::parse(&format!("{}\0", "a".repeat(300)))),
            message(PageToken::parse(&format!("{}\0", "a".repeat(300)))),
        ] {
            assert!(text.contains(&"a".repeat(40)), "{text}");
            assert!(!text.contains(&"a".repeat(41)), "{text}");
        }
    }

    #[test]
    fn a_refusal_escapes_control_characters_in_the_value() {
        let text = message(ProjectKey::parse("x\u{1b}[31m\nred"));
        assert!(text.contains("\\u{1b}"), "{text}");
        assert!(text.contains("\\n"), "{text}");
        assert!(!text.contains('\u{1b}') && !text.contains('\n'), "{text:?}");
    }
}
