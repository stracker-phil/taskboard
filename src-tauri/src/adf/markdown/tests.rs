use serde_json::{Value, json};

use super::to_markdown;

fn doc(content: Value) -> Value {
    json!({ "type": "doc", "version": 1, "content": content })
}

fn md(content: Value) -> String {
    to_markdown(&doc(content)).unwrap()
}

fn text(s: &str) -> Value {
    json!({ "type": "text", "text": s })
}

fn marked(s: &str, marks: Value) -> Value {
    json!({ "type": "text", "text": s, "marks": marks })
}

fn para(inline: Value) -> Value {
    json!({ "type": "paragraph", "content": inline })
}

fn p(s: &str) -> Value {
    para(json!([text(s)]))
}

fn item(blocks: Value) -> Value {
    json!({ "type": "listItem", "content": blocks })
}

fn heading(level: u64, s: &str) -> Value {
    json!({ "type": "heading", "attrs": { "level": level }, "content": [text(s)] })
}

#[test]
fn a_value_that_is_no_document_gives_none() {
    for value in [
        json!(null),
        json!({ "type": "paragraph", "content": [] }),
        json!({ "type": "doc" }),
        json!({ "type": "doc", "content": "x" }),
    ] {
        assert_eq!(to_markdown(&value), None, "{value}");
    }
}

#[test]
fn an_empty_document_gives_empty_text() {
    assert_eq!(to_markdown(&doc(json!([]))), Some(String::new()));
}

#[test]
fn paragraphs_are_separated_by_a_blank_line() {
    assert_eq!(md(json!([p("One"), p("Two")])), "One\n\nTwo");
}

#[test]
fn headings_sit_two_levels_lower_and_stop_at_six() {
    for (level, expected) in [
        (1, "### Title"),
        (2, "#### Title"),
        (3, "##### Title"),
        (4, "###### Title"),
        (6, "###### Title"),
    ] {
        assert_eq!(md(json!([heading(level, "Title")])), expected, "{level}");
    }
}

#[test]
fn a_bullet_list_has_one_dash_per_item() {
    let list = json!({ "type": "bulletList", "content": [
        item(json!([p("One")])),
        item(json!([p("Two")])),
    ]});
    assert_eq!(md(json!([list])), "- One\n- Two");
}

#[test]
fn a_nested_list_is_indented_under_its_item() {
    let inner = json!({ "type": "bulletList", "content": [item(json!([p("Inner")]))] });
    let list = json!({ "type": "bulletList", "content": [
        item(json!([p("One"), inner])),
        item(json!([p("Two")])),
    ]});
    assert_eq!(md(json!([list])), "- One\n  - Inner\n- Two");
}

#[test]
fn an_ordered_list_counts_from_its_start() {
    let content = json!([item(json!([p("a")])), item(json!([p("b")]))]);
    let default = json!({ "type": "orderedList", "content": content });
    let from_three = json!({ "type": "orderedList", "attrs": { "order": 3 }, "content": content });

    assert_eq!(md(json!([default])), "1. a\n2. b");
    assert_eq!(md(json!([from_three])), "3. a\n4. b");
}

#[test]
fn a_task_list_shows_open_and_done_boxes() {
    let list = json!({ "type": "taskList", "content": [
        { "type": "taskItem", "attrs": { "state": "TODO" }, "content": [text("Buy")] },
        { "type": "taskItem", "attrs": { "state": "DONE" }, "content": [text("Ship")] },
    ]});
    assert_eq!(md(json!([list])), "- [ ] Buy\n- [x] Ship");
}

#[test]
fn a_code_block_is_fenced_with_its_language() {
    let code = json!({ "type": "codeBlock", "attrs": { "language": "rust" },
        "content": [text("let a = 1;")] });
    let plain = json!({ "type": "codeBlock", "content": [text("echo hi")] });

    assert_eq!(md(json!([code])), "```rust\nlet a = 1;\n```");
    assert_eq!(md(json!([plain])), "```\necho hi\n```");
}

#[test]
fn a_code_fence_is_longer_than_the_backticks_inside() {
    let code = json!({ "type": "codeBlock", "attrs": { "language": "md" },
        "content": [text("```x```")] });
    assert_eq!(md(json!([code])), "````md\n```x```\n````");
}

#[test]
fn blockquotes_and_panels_are_quoted_line_by_line() {
    for kind in ["blockquote", "panel"] {
        let quote = json!({ "type": kind, "content": [p("One"), p("Two")] });
        assert_eq!(md(json!([quote])), "> One\n>\n> Two", "{kind}");
    }
}

#[test]
fn a_rule_is_three_dashes() {
    let rule = json!({ "type": "rule" });
    assert_eq!(md(json!([p("a"), rule, p("b")])), "a\n\n---\n\nb");
}

#[test]
fn a_table_is_a_pipe_table_with_a_header_separator_and_escaped_pipes() {
    let cell = |kind: &str, s: &str| json!({ "type": kind, "content": [p(s)] });
    let table = json!({ "type": "table", "content": [
        { "type": "tableRow", "content": [cell("tableHeader", "Name"), cell("tableHeader", "Note")] },
        { "type": "tableRow", "content": [cell("tableCell", "Ann"), cell("tableCell", "a|b")] },
    ]});
    assert_eq!(
        md(json!([table])),
        "| Name | Note |\n| --- | --- |\n| Ann | a\\|b |"
    );
}

#[test]
fn marks_wrap_the_text() {
    for (marks, expected) in [
        (json!([{ "type": "strong" }]), "**word**"),
        (json!([{ "type": "em" }]), "*word*"),
        (json!([{ "type": "code" }]), "`word`"),
        (json!([{ "type": "strike" }]), "~~word~~"),
        (
            json!([{ "type": "link", "attrs": { "href": "https://example.com/a" } }]),
            "[word](https://example.com/a)",
        ),
        (
            json!([{ "type": "strong" }, { "type": "link", "attrs": { "href": "https://example.com" } }]),
            "[**word**](https://example.com)",
        ),
    ] {
        assert_eq!(
            md(json!([para(json!([marked("word", marks.clone())]))])),
            expected,
            "{marks}"
        );
    }
}

#[test]
fn spaces_at_the_ends_of_marked_text_stay_outside_the_marks() {
    let line = para(json!([
        text("Hello"),
        marked(" world ", json!([{ "type": "strong" }])),
        text("now"),
    ]));
    assert_eq!(md(json!([line])), "Hello **world** now");
}

#[test]
fn a_mention_is_an_at_sign_and_the_name() {
    for shown in ["@Ann Example", "Ann Example"] {
        let mention = json!({ "type": "mention", "attrs": { "id": "u1", "text": shown } });
        assert_eq!(
            md(json!([para(json!([text("Hi "), mention]))])),
            "Hi @Ann Example",
            "{shown}"
        );
    }
}

#[test]
fn an_emoji_is_its_character_else_its_short_name() {
    let with_text = json!({ "type": "emoji", "attrs": { "shortName": ":smile:", "text": "😄" } });
    let name_only = json!({ "type": "emoji", "attrs": { "shortName": ":smile:" } });

    assert_eq!(md(json!([para(json!([with_text]))])), "😄");
    assert_eq!(md(json!([para(json!([name_only]))])), ":smile:");
}

#[test]
fn a_date_is_shown_as_a_day() {
    let date = json!({ "type": "date", "attrs": { "timestamp": "1767225600000" } });
    assert_eq!(md(json!([para(json!([date]))])), "2026-01-01");
}

#[test]
fn an_inline_card_is_a_link_to_its_address() {
    let card = json!({ "type": "inlineCard", "attrs": { "url": "https://example.com/page" } });
    assert_eq!(
        md(json!([para(json!([card]))])),
        "[https://example.com/page](https://example.com/page)"
    );
}

#[test]
fn an_attachment_is_a_text_placeholder_with_its_name() {
    let media =
        json!({ "type": "media", "attrs": { "type": "file", "id": "m1", "alt": "diagram.png" } });
    let single = json!({ "type": "mediaSingle", "content": [media] });
    assert_eq!(md(json!([single])), "[attachment: diagram.png]");
}

#[test]
fn a_hard_break_ends_the_line_with_a_backslash() {
    let line = para(json!([text("a"), { "type": "hardBreak" }, text("b")]));
    assert_eq!(md(json!([line])), "a\\\nb");
}
