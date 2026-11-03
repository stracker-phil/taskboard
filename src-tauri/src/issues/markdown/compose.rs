//! The issue as Markdown, from plain inputs. Pure.

use chrono::DateTime;
use chrono_tz::Tz;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;

use crate::adf;
use crate::text::html_to_paragraphs;

/// A text as the cache holds it.
#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    /// An ADF document; a value that isn't one gives no content.
    Adf(Value),
    /// Jira's rendered HTML, where no ADF is stored.
    Html(String),
    /// A field's displayed value.
    Plain(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownComment {
    /// The author's name; "Someone" when unknown.
    pub author: Option<String>,
    /// UTC, RFC 3339.
    pub at: String,
    pub body: Option<Content>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownIssue {
    /// The issue's address on the Jira site (`browse_url`).
    pub source: Option<String>,
    pub summary: String,
    pub status: String,
    pub assignees: Vec<String>,
    pub priority: Option<String>,
    pub description: Option<Content>,
    /// Field name and content, in the Issue view's order.
    pub fields: Vec<(String, Content)>,
    /// Pull request addresses, listed in the front matter.
    pub pull_requests: Vec<String>,
    /// The other web links: title and address; a blank title shows the address.
    pub web_links: Vec<(String, String)>,
    /// In any order: they are written oldest first.
    pub comments: Vec<MarkdownComment>,
}

/// The issue as Markdown; comment times are shown in `tz`. Sections without content are left out.
pub fn compose(issue: &MarkdownIssue, tz: Tz) -> String {
    let mut parts = vec![front_matter(issue), format!("# {}", issue.summary.trim())];
    let mut section = |heading: &str, body: Option<String>| {
        if let Some(body) = body.filter(|b| !b.is_empty()) {
            parts.push(format!("## {}\n\n{body}", heading.trim()));
        }
    };
    section("Description", issue.description.as_ref().and_then(markdown));
    for (name, content) in &issue.fields {
        if !is_front_matter_field(name, content) {
            section(name, markdown(content));
        }
    }
    section("Weblinks", web_links(&issue.web_links));
    section("Comments", comments(&issue.comments, tz));
    parts.join("\n\n") + "\n"
}

fn front_matter(issue: &MarkdownIssue) -> String {
    let pull_requests: Vec<&str> = issue
        .pull_requests
        .iter()
        .filter_map(|url| non_blank(url))
        .collect();
    let pr = match pull_requests.as_slice() {
        [] => None,
        [one] => meta("pr", Some(one)),
        several => Some(
            std::iter::once("pr:".to_string())
                .chain(several.iter().map(|url| format!("  - {}", yaml(url))))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    };
    let developer = issue
        .fields
        .iter()
        .find_map(|(name, content)| match content {
            Content::Plain(value) if is_front_matter_field(name, content) => {
                meta("developer", Some(value))
            }
            _ => None,
        });
    let lines = [
        meta("source", issue.source.as_deref()),
        pr,
        meta("status", Some(&issue.status)),
        meta("assignee", Some(&issue.assignees.join(", "))),
        developer,
        meta("priority", issue.priority.as_deref()),
    ];
    let mut front = vec!["---".to_string()];
    front.extend(lines.into_iter().flatten());
    front.push("---".to_string());
    front.join("\n")
}

/// `key: value`; `None` for a blank value.
fn meta(key: &str, value: Option<&str>) -> Option<String> {
    value
        .and_then(non_blank)
        .map(|value| format!("{key}: {}", yaml(value)))
}

/// The Developer field, a person, goes in the front matter next to the assignee instead of a section.
fn is_front_matter_field(name: &str, content: &Content) -> bool {
    matches!(content, Content::Plain(_)) && name.trim().eq_ignore_ascii_case("developer")
}

/// `<site>/browse/<key>`, as the page builds it; `None` without a site.
pub fn browse_url(site_url: &str, key: &str) -> Option<String> {
    const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'!')
        .remove(b'~')
        .remove(b'*')
        .remove(b'\'')
        .remove(b'(')
        .remove(b')');
    let site = site_url.trim().trim_end_matches('/');
    (!site.is_empty()).then(|| format!("{site}/browse/{}", utf8_percent_encode(key, COMPONENT)))
}

/// The content as Markdown; `None` when it has none.
fn markdown(content: &Content) -> Option<String> {
    let text = match content {
        Content::Adf(doc) => adf::to_markdown(doc)?,
        Content::Html(html) => html_to_paragraphs(html),
        Content::Plain(text) => text.trim().to_string(),
    };
    (!text.is_empty()).then_some(text)
}

fn web_links(links: &[(String, String)]) -> Option<String> {
    let lines: Vec<String> = links
        .iter()
        .filter_map(|(title, url)| {
            let url = non_blank(url)?;
            let title = non_blank(title).unwrap_or(url);
            Some(format!("- [{title}]({url})"))
        })
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn comments(comments: &[MarkdownComment], tz: Tz) -> Option<String> {
    let mut sorted: Vec<&MarkdownComment> = comments.iter().collect();
    sorted.sort_by(|a, b| a.at.cmp(&b.at));
    let blocks: Vec<String> = sorted
        .into_iter()
        .map(|comment| {
            let author = comment
                .author
                .as_deref()
                .and_then(non_blank)
                .unwrap_or("Someone");
            let heading = format!("### {author} · {}", local_time(&comment.at, tz));
            match comment.body.as_ref().and_then(markdown) {
                Some(body) => format!("{heading}\n\n{body}"),
                None => heading,
            }
        })
        .collect();
    (!blocks.is_empty()).then(|| blocks.join("\n\n"))
}

/// `2026-10-28 11:42` in `tz`; text that isn't RFC 3339 is returned unchanged.
fn local_time(at: &str, tz: Tz) -> String {
    match DateTime::parse_from_rfc3339(at) {
        Ok(time) => time.with_timezone(&tz).format("%Y-%m-%d %H:%M").to_string(),
        Err(_) => at.to_string(),
    }
}

/// The text trimmed; `None` when that leaves nothing.
fn non_blank(text: &str) -> Option<&str> {
    Some(text.trim()).filter(|t| !t.is_empty())
}

/// A YAML scalar: plain when YAML reads it back as the same string, else double-quoted.
fn yaml(value: &str) -> String {
    const SPECIAL_START: &[char] = &[
        '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@',
        '`',
    ];
    const RESERVED: &[&str] = &[
        "true", "false", "yes", "no", "on", "off", "null", "~", "y", "n",
    ];
    let plain = !value.starts_with(SPECIAL_START)
        && !value.contains(": ")
        && !value.contains(" #")
        && !value.ends_with(':')
        && !value.contains(['\n', '\r', '\t'])
        && !RESERVED.contains(&value.to_ascii_lowercase().as_str())
        && value.parse::<f64>().is_err();
    if plain {
        return value.to_string();
    }
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn issue() -> MarkdownIssue {
        MarkdownIssue {
            source: None,
            summary: "Fix checkout".into(),
            status: String::new(),
            assignees: Vec::new(),
            priority: None,
            description: None,
            fields: Vec::new(),
            pull_requests: Vec::new(),
            web_links: Vec::new(),
            comments: Vec::new(),
        }
    }

    fn plain(text: &str) -> Content {
        Content::Plain(text.into())
    }

    fn comment(author: Option<&str>, at: &str, body: Option<&str>) -> MarkdownComment {
        MarkdownComment {
            author: author.map(Into::into),
            at: at.into(),
            body: body.map(plain),
        }
    }

    #[test]
    fn a_bare_issue_is_empty_front_matter_and_the_title() {
        assert_eq!(compose(&issue(), Tz::UTC), "---\n---\n\n# Fix checkout\n");
    }

    #[test]
    fn front_matter_lists_source_status_assignees_and_priority() {
        let full = MarkdownIssue {
            source: Some("https://example.com/browse/NW-1".into()),
            status: "In Progress".into(),
            assignees: vec!["Ann Example".into(), "Bob Demo".into()],
            priority: Some("High".into()),
            ..issue()
        };
        assert_eq!(
            compose(&full, Tz::UTC),
            "---\nsource: https://example.com/browse/NW-1\nstatus: In Progress\n\
             assignee: Ann Example, Bob Demo\npriority: High\n---\n\n# Fix checkout\n"
        );
    }

    #[test]
    fn blank_front_matter_values_are_left_out() {
        let blank = MarkdownIssue {
            source: Some("  ".into()),
            status: "Open".into(),
            priority: Some(String::new()),
            ..issue()
        };
        assert_eq!(
            compose(&blank, Tz::UTC),
            "---\nstatus: Open\n---\n\n# Fix checkout\n"
        );
    }

    #[test]
    fn front_matter_values_that_yaml_would_misread_are_quoted() {
        for (status, line) in [
            ("Done", "status: Done"),
            ("Fix: now", "status: \"Fix: now\""),
            ("yes", "status: \"yes\""),
            ("Null", "status: \"Null\""),
            ("42", "status: \"42\""),
            ("#hot", "status: \"#hot\""),
            ("- item", "status: \"- item\""),
            ("a #b", "status: \"a #b\""),
            ("ends:", "status: \"ends:\""),
            ("say \"hi\" \\ bye", "status: say \"hi\" \\ bye"),
            ("@ann", "status: \"@ann\""),
        ] {
            let text = compose(
                &MarkdownIssue {
                    status: status.into(),
                    ..issue()
                },
                Tz::UTC,
            );
            assert_eq!(text.lines().nth(1), Some(line), "{status}");
        }
    }

    #[test]
    fn quoted_front_matter_values_escape_quotes_and_backslashes() {
        let text = compose(
            &MarkdownIssue {
                status: "a: \"b\" \\".into(),
                ..issue()
            },
            Tz::UTC,
        );
        assert_eq!(text.lines().nth(1), Some(r#"status: "a: \"b\" \\""#));
    }

    #[test]
    fn description_and_fields_become_sections_in_order() {
        let doc = json!({ "type": "doc", "version": 1, "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "From ADF" }] }
        ]});
        let full = MarkdownIssue {
            description: Some(Content::Adf(doc)),
            fields: vec![
                ("Notes".into(), plain("  Check the cart  ")),
                (
                    "Rendered".into(),
                    Content::Html("<p>One</p><p>Two</p>".into()),
                ),
            ],
            ..issue()
        };
        assert_eq!(
            compose(&full, Tz::UTC),
            "---\n---\n\n# Fix checkout\n\n## Description\n\nFrom ADF\n\n## Notes\n\nCheck the cart\n\n\
             ## Rendered\n\nOne\n\nTwo\n"
        );
    }

    #[test]
    fn sections_without_content_are_dropped() {
        let empty = MarkdownIssue {
            description: Some(Content::Adf(json!({ "type": "paragraph" }))),
            fields: vec![
                ("Blank".into(), plain("   ")),
                ("Html".into(), Content::Html("<p> </p>".into())),
                (
                    "Doc".into(),
                    Content::Adf(json!({ "type": "doc", "content": [] })),
                ),
            ],
            ..issue()
        };
        assert_eq!(compose(&empty, Tz::UTC), "---\n---\n\n# Fix checkout\n");
    }

    #[test]
    fn weblinks_are_a_list_with_the_address_as_a_missing_title() {
        let linked = MarkdownIssue {
            web_links: vec![
                ("Docs".into(), "https://example.com/docs".into()),
                ("  ".into(), "https://example.com/x".into()),
                ("Gone".into(), "  ".into()),
            ],
            ..issue()
        };
        assert_eq!(
            compose(&linked, Tz::UTC),
            "---\n---\n\n# Fix checkout\n\n## Weblinks\n\n- [Docs](https://example.com/docs)\n\
             - [https://example.com/x](https://example.com/x)\n"
        );
    }

    #[test]
    fn one_pull_request_is_a_pr_line_right_after_the_source() {
        let reviewed = MarkdownIssue {
            source: Some("https://example.com/browse/NW-1".into()),
            status: "In Review".into(),
            pull_requests: vec!["https://github.com/example/shop/pull/481".into()],
            ..issue()
        };
        assert_eq!(
            compose(&reviewed, Tz::UTC),
            "---\nsource: https://example.com/browse/NW-1\n\
             pr: https://github.com/example/shop/pull/481\nstatus: In Review\n---\n\n# Fix checkout\n"
        );
    }

    #[test]
    fn several_pull_requests_are_a_pr_list_in_order_before_the_status() {
        let reviewed = MarkdownIssue {
            source: Some("https://example.com/browse/NW-1".into()),
            status: "In Review".into(),
            pull_requests: vec![
                "https://github.com/example/shop/pull/481".into(),
                "  ".into(),
                "https://github.com/example/shop/pull/377".into(),
            ],
            ..issue()
        };
        assert_eq!(
            compose(&reviewed, Tz::UTC),
            "---\nsource: https://example.com/browse/NW-1\npr:\n\
             \x20 - https://github.com/example/shop/pull/481\n\
             \x20 - https://github.com/example/shop/pull/377\nstatus: In Review\n---\n\n# Fix checkout\n"
        );
    }

    #[test]
    fn blank_pull_requests_leave_no_pr_line() {
        for pull_requests in [vec![], vec![String::new(), "  ".into()]] {
            let blank = MarkdownIssue {
                status: "Open".into(),
                pull_requests: pull_requests.clone(),
                ..issue()
            };
            assert_eq!(
                compose(&blank, Tz::UTC),
                "---\nstatus: Open\n---\n\n# Fix checkout\n",
                "{pull_requests:?}"
            );
        }
    }

    #[test]
    fn a_plain_developer_field_is_front_matter_between_assignee_and_priority() {
        for name in ["Developer", "  developer ", "DEVELOPER"] {
            let developed = MarkdownIssue {
                status: "Open".into(),
                assignees: vec!["Ann Example".into()],
                priority: Some("High".into()),
                fields: vec![(name.into(), plain("Bob Demo"))],
                ..issue()
            };
            assert_eq!(
                compose(&developed, Tz::UTC),
                "---\nstatus: Open\nassignee: Ann Example\ndeveloper: Bob Demo\npriority: High\n---\n\n\
                 # Fix checkout\n",
                "{name:?}"
            );
        }
    }

    #[test]
    fn a_developer_field_that_is_not_plain_stays_a_section() {
        let doc = json!({ "type": "doc", "version": 1, "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": "Bob Demo" }] }
        ]});
        let developed = MarkdownIssue {
            fields: vec![("Developer".into(), Content::Adf(doc))],
            ..issue()
        };
        assert_eq!(
            compose(&developed, Tz::UTC),
            "---\n---\n\n# Fix checkout\n\n## Developer\n\nBob Demo\n"
        );
    }

    #[test]
    fn other_plain_fields_stay_sections_beside_the_developer() {
        let developed = MarkdownIssue {
            fields: vec![
                ("Reviewer".into(), plain("Cy Sample")),
                ("Developer".into(), plain("Bob Demo")),
            ],
            ..issue()
        };
        assert_eq!(
            compose(&developed, Tz::UTC),
            "---\ndeveloper: Bob Demo\n---\n\n# Fix checkout\n\n## Reviewer\n\nCy Sample\n"
        );
    }

    #[test]
    fn pull_requests_alone_make_no_weblinks_section() {
        let reviewed = MarkdownIssue {
            pull_requests: vec!["https://github.com/example/shop/pull/481".into()],
            ..issue()
        };
        assert!(!compose(&reviewed, Tz::UTC).contains("## Weblinks"));
    }

    #[test]
    fn weblinks_list_only_the_links_that_are_not_pull_requests() {
        let linked = MarkdownIssue {
            pull_requests: vec!["https://github.com/example/shop/pull/481".into()],
            web_links: vec![("Docs".into(), "https://example.com/docs".into())],
            ..issue()
        };
        assert_eq!(
            compose(&linked, Tz::UTC),
            "---\npr: https://github.com/example/shop/pull/481\n---\n\n# Fix checkout\n\n\
             ## Weblinks\n\n- [Docs](https://example.com/docs)\n"
        );
    }

    #[test]
    fn comments_are_written_oldest_first_in_the_given_time_zone() {
        let talked = MarkdownIssue {
            comments: vec![
                comment(None, "2026-07-02T08:00:00Z", Some("Second")),
                comment(Some("Ann Example"), "2026-07-01T22:30:00Z", Some("First")),
            ],
            ..issue()
        };
        assert_eq!(
            compose(&talked, Tz::Europe__Berlin),
            "---\n---\n\n# Fix checkout\n\n## Comments\n\n\
             ### Ann Example · 2026-07-02 00:30\n\nFirst\n\n### Someone · 2026-07-02 10:00\n\nSecond\n"
        );
    }

    #[test]
    fn a_blank_author_is_someone_and_a_comment_without_text_keeps_its_heading() {
        let talked = MarkdownIssue {
            comments: vec![
                comment(Some("  "), "2026-07-01T10:00:00Z", None),
                comment(Some("Bob Demo"), "not a time", None),
            ],
            ..issue()
        };
        assert_eq!(
            compose(&talked, Tz::UTC),
            "---\n---\n\n# Fix checkout\n\n## Comments\n\n\
             ### Someone · 2026-07-01 10:00\n\n### Bob Demo · not a time\n"
        );
    }

    #[test]
    fn the_text_ends_with_a_single_newline() {
        let full = MarkdownIssue {
            description: Some(plain("Text")),
            comments: vec![comment(None, "2026-07-01T10:00:00Z", Some("Hi"))],
            ..issue()
        };
        for text in [compose(&issue(), Tz::UTC), compose(&full, Tz::UTC)] {
            assert!(text.ends_with('\n') && !text.ends_with("\n\n"), "{text:?}");
        }
    }

    #[test]
    fn the_browse_address_trims_the_site_and_encodes_the_key() {
        for (site, key, expected) in [
            (
                "https://example.com",
                "NW-1",
                Some("https://example.com/browse/NW-1"),
            ),
            (
                "  https://example.com/// ",
                "NW-1",
                Some("https://example.com/browse/NW-1"),
            ),
            (
                "https://example.com",
                "A B/1",
                Some("https://example.com/browse/A%20B%2F1"),
            ),
            ("", "NW-1", None),
            ("  /  ", "NW-1", None),
        ] {
            assert_eq!(
                browse_url(site, key).as_deref(),
                expected,
                "{site:?} {key:?}"
            );
        }
    }
}
