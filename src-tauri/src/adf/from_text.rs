//! Plain text → an ADF document, for comments typed in Taskboard: a blank line starts a paragraph, a line break
//! is a hard break, and `http(s)://` addresses become links.

use serde_json::{Value, json};

pub fn from_plain_text(text: &str) -> Value {
    let paragraphs: Vec<Value> = paragraphs(text)
        .into_iter()
        .map(|lines| json!({ "type": "paragraph", "content": paragraph_content(&lines) }))
        .collect();
    json!({ "type": "doc", "version": 1, "content": paragraphs })
}

/// Non-blank lines grouped by the blank lines between them.
fn paragraphs(text: &str) -> Vec<Vec<&str>> {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    lines
        .split(|line| line.trim().is_empty())
        .filter(|group| !group.is_empty())
        .map(<[&str]>::to_vec)
        .collect()
}

fn paragraph_content(lines: &[&str]) -> Vec<Value> {
    let mut content = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            content.push(json!({ "type": "hardBreak" }));
        }
        content.extend(inline(line));
    }
    content
}

/// Text nodes, with a link mark on each address.
fn inline(line: &str) -> Vec<Value> {
    let mut nodes = Vec::new();
    let mut rest = line;
    while let Some(start) = next_link(rest) {
        let (before, from_link) = rest.split_at(start);
        let end = link_end(from_link);
        let (url, after) = from_link.split_at(end);
        push_text(&mut nodes, before);
        nodes.push(json!({
            "type": "text",
            "text": url,
            "marks": [{ "type": "link", "attrs": { "href": url } }],
        }));
        rest = after;
    }
    push_text(&mut nodes, rest);
    nodes
}

fn next_link(text: &str) -> Option<usize> {
    [text.find("https://"), text.find("http://")]
        .into_iter()
        .flatten()
        .min()
}

/// The address runs to the next space; closing punctuation after it belongs to the sentence.
fn link_end(from_link: &str) -> usize {
    let word = from_link
        .find(char::is_whitespace)
        .unwrap_or(from_link.len());
    from_link[..word]
        .trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '"', '\''])
        .len()
}

fn push_text(nodes: &mut Vec<Value>, text: &str) {
    if !text.is_empty() {
        nodes.push(json!({ "type": "text", "text": text }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(text: &str) -> Value {
        json!({ "type": "text", "text": text })
    }

    fn link(url: &str) -> Value {
        json!({
            "type": "text",
            "text": url,
            "marks": [{ "type": "link", "attrs": { "href": url } }],
        })
    }

    fn hard_break() -> Value {
        json!({ "type": "hardBreak" })
    }

    fn doc(paragraphs: Vec<Vec<Value>>) -> Value {
        let content: Vec<Value> = paragraphs
            .into_iter()
            .map(|content| json!({ "type": "paragraph", "content": content }))
            .collect();
        json!({ "type": "doc", "version": 1, "content": content })
    }

    #[test]
    fn a_line_becomes_one_paragraph_in_a_version_one_document() {
        assert_eq!(
            from_plain_text("Checked on staging"),
            doc(vec![vec![text("Checked on staging")]])
        );
    }

    #[test]
    fn blank_lines_split_paragraphs_and_whitespace_only_lines_count_as_blank() {
        for input in [
            "First\n\nSecond",
            "First\n   \nSecond",
            "First\n\t\n\n\nSecond",
            "\n\nFirst\n\nSecond\n\n",
            "First\r\n\r\nSecond",
        ] {
            assert_eq!(
                from_plain_text(input),
                doc(vec![vec![text("First")], vec![text("Second")]]),
                "{input:?}"
            );
        }
    }

    #[test]
    fn single_line_breaks_become_hard_breaks_and_trailing_spaces_are_trimmed() {
        assert_eq!(
            from_plain_text("one  \ntwo\t\nthree"),
            doc(vec![vec![
                text("one"),
                hard_break(),
                text("two"),
                hard_break(),
                text("three")
            ]])
        );
    }

    #[test]
    fn text_without_content_is_a_document_without_paragraphs() {
        for input in ["", "   ", "\n\n \n"] {
            assert_eq!(
                from_plain_text(input),
                json!({ "type": "doc", "version": 1, "content": [] }),
                "{input:?}"
            );
        }
    }

    #[test]
    fn web_addresses_become_text_nodes_with_a_link_mark() {
        assert_eq!(
            from_plain_text("See https://example.com/docs?page=2#top for details"),
            doc(vec![vec![
                text("See "),
                link("https://example.com/docs?page=2#top"),
                text(" for details"),
            ]])
        );
        assert_eq!(
            from_plain_text("http://example.com"),
            doc(vec![vec![link("http://example.com")]])
        );
    }

    #[test]
    fn closing_punctuation_stays_outside_the_link() {
        for (input, url, after) in [
            ("Done: https://example.com.", "https://example.com", "."),
            ("(see https://example.com/a)", "https://example.com/a", ")"),
            ("https://example.com/a,", "https://example.com/a", ","),
            ("https://example.com/a?!", "https://example.com/a", "?!"),
            ("\"https://example.com/a\"", "https://example.com/a", "\""),
            ("https://example.com/a;:'", "https://example.com/a", ";:'"),
        ] {
            let document = from_plain_text(input);
            let nodes = document["content"][0]["content"].as_array().unwrap();

            let found = nodes
                .iter()
                .find(|node| node["marks"][0]["type"] == "link")
                .unwrap_or_else(|| panic!("no link in {input:?}"));
            assert_eq!(found, &link(url), "{input:?}");
            assert_eq!(nodes.last().unwrap(), &text(after), "{input:?}");
        }
    }

    #[test]
    fn several_addresses_on_one_line_each_get_their_own_link() {
        assert_eq!(
            from_plain_text("https://example.com/a and http://example.org/b."),
            doc(vec![vec![
                link("https://example.com/a"),
                text(" and "),
                link("http://example.org/b"),
                text("."),
            ]])
        );
    }

    #[test]
    fn an_address_ends_at_whitespace_and_a_line_break_ends_it_too() {
        assert_eq!(
            from_plain_text("https://example.com/a\nhttps://example.com/b"),
            doc(vec![vec![
                link("https://example.com/a"),
                hard_break(),
                link("https://example.com/b"),
            ]])
        );
    }

    #[test]
    fn text_that_only_looks_like_an_address_stays_plain() {
        assert_eq!(
            from_plain_text("ftp://example.com and example.com"),
            doc(vec![vec![text("ftp://example.com and example.com")]])
        );
    }
}
