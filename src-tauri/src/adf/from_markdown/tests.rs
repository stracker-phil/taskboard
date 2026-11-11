use super::*;
use serde_json::{Value, json};

fn doc(content: Value) -> Value {
    json!({ "type": "doc", "version": 1, "content": content })
}

fn text(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn marked(text: &str, marks: Value) -> Value {
    json!({ "type": "text", "text": text, "marks": marks })
}

fn linked(text: &str, href: &str) -> Value {
    marked(text, json!([{ "type": "link", "attrs": { "href": href } }]))
}

fn paragraph(content: Value) -> Value {
    json!({ "type": "paragraph", "content": content })
}

fn list_item(content: Value) -> Value {
    json!({ "type": "listItem", "content": content })
}

fn mark_types(node: &Value) -> Vec<&str> {
    let mut types: Vec<&str> = node["marks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["type"].as_str().unwrap())
        .collect();
    types.sort_unstable();
    types
}

fn assert_no_empty_text(node: &Value, input: &str) {
    if node["type"] == "text" {
        assert!(!node["text"].as_str().unwrap().is_empty(), "{input:?}");
    }
    for child in node["content"].as_array().into_iter().flatten() {
        assert_no_empty_text(child, input);
    }
}

#[test]
fn a_line_is_one_paragraph_in_a_document() {
    assert_eq!(
        from_markdown("Checked on staging"),
        doc(json!([paragraph(json!([text("Checked on staging")]))]))
    );
}

#[test]
fn empty_or_blank_input_gives_a_document_without_content() {
    for input in ["", "   ", "\n\n", " \t\n  \n"] {
        assert_eq!(from_markdown(input), doc(json!([])), "{input:?}");
    }
}

#[test]
fn blank_lines_split_paragraphs() {
    let expected = doc(json!([
        paragraph(json!([text("one")])),
        paragraph(json!([text("two")])),
    ]));
    for input in [
        "one\n\ntwo",
        "one\n  \t\ntwo",
        "one\r\n\r\ntwo",
        "one\n\n\n\ntwo",
    ] {
        assert_eq!(from_markdown(input), expected, "{input:?}");
    }
}

#[test]
fn a_line_break_inside_a_paragraph_is_a_hard_break() {
    let expected = doc(json!([paragraph(json!([
        text("one"),
        { "type": "hardBreak" },
        text("two"),
    ]))]));
    for input in ["one\ntwo", "one\r\ntwo", "one  \ntwo", "one\\\ntwo"] {
        assert_eq!(from_markdown(input), expected, "{input:?}");
    }
}

#[test]
fn a_bare_address_becomes_a_link() {
    assert_eq!(
        from_markdown("see https://example.com/docs now"),
        doc(json!([paragraph(json!([
            text("see "),
            linked("https://example.com/docs", "https://example.com/docs"),
            text(" now"),
        ]))]))
    );
    assert_eq!(
        from_markdown("http://example.com"),
        doc(json!([paragraph(json!([linked(
            "http://example.com",
            "http://example.com"
        )]))]))
    );
}

#[test]
fn closing_punctuation_stays_outside_a_bare_address() {
    for (input, address, after) in [
        ("go to https://example.com/a.", "https://example.com/a", "."),
        ("(see https://example.com/a)", "https://example.com/a", ")"),
        ("is it https://example.com/a?", "https://example.com/a", "?"),
        (
            "see https://example.com/a, ok",
            "https://example.com/a",
            ", ok",
        ),
        ("see https://example.com/a).", "https://example.com/a", ")."),
    ] {
        let result = from_markdown(input);
        let nodes = result["content"][0]["content"].as_array().unwrap();
        let link = nodes.iter().find(|n| n.get("marks").is_some()).unwrap();
        assert_eq!(*link, linked(address, address), "{input}");
        assert_eq!(nodes.last().unwrap(), &text(after), "{input}");
    }
}

#[test]
fn several_addresses_on_a_line_each_become_a_link() {
    assert_eq!(
        from_markdown("https://example.com/1 and http://example.com/2"),
        doc(json!([paragraph(json!([
            linked("https://example.com/1", "https://example.com/1"),
            text(" and "),
            linked("http://example.com/2", "http://example.com/2"),
        ]))]))
    );
}

#[test]
fn emphasis_strong_and_strike_become_marks() {
    for (input, mark) in [
        ("**bold**", "strong"),
        ("__bold__", "strong"),
        ("_it_", "em"),
        ("*it*", "em"),
        ("~~gone~~", "strike"),
    ] {
        let result = from_markdown(input);
        let node = &result["content"][0]["content"][0];
        assert_eq!(node["marks"], json!([{ "type": mark }]), "{input}");
    }
    assert_eq!(
        from_markdown("a **b** c"),
        doc(json!([paragraph(json!([
            text("a "),
            marked("b", json!([{ "type": "strong" }])),
            text(" c"),
        ]))]))
    );
}

#[test]
fn nested_emphasis_carries_both_marks() {
    let result = from_markdown("***both***");
    let content = result["content"][0]["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["text"], "both");
    assert_eq!(mark_types(&content[0]), ["em", "strong"]);
}

#[test]
fn inline_code_becomes_a_code_mark() {
    assert_eq!(
        from_markdown("run `cargo test` now"),
        doc(json!([paragraph(json!([
            text("run "),
            marked("cargo test", json!([{ "type": "code" }])),
            text(" now"),
        ]))]))
    );
}

#[test]
fn code_drops_strong_and_emphasis() {
    for input in ["**`x`**", "_`x`_", "~~`x`~~"] {
        assert_eq!(
            from_markdown(input),
            doc(json!([paragraph(json!([marked(
                "x",
                json!([{ "type": "code" }])
            )]))])),
            "{input}"
        );
    }
}

#[test]
fn code_keeps_a_link() {
    assert_eq!(
        from_markdown("[`x`](https://example.com)"),
        doc(json!([paragraph(json!([marked(
            "x",
            json!([
                { "type": "link", "attrs": { "href": "https://example.com" } },
                { "type": "code" },
            ])
        )]))]))
    );
}

#[test]
fn a_markdown_link_puts_a_link_mark_on_its_text() {
    assert_eq!(
        from_markdown("open [the board](https://example.com/board) today"),
        doc(json!([paragraph(json!([
            text("open "),
            linked("the board", "https://example.com/board"),
            text(" today"),
        ]))]))
    );
}

#[test]
fn a_link_keeps_the_marks_of_its_text() {
    assert_eq!(
        from_markdown("[**bold**](https://example.com)"),
        doc(json!([paragraph(json!([marked(
            "bold",
            json!([
                { "type": "strong" },
                { "type": "link", "attrs": { "href": "https://example.com" } },
            ])
        )]))]))
    );
}

#[test]
fn a_link_to_a_non_web_target_keeps_its_text_without_a_link() {
    for input in [
        "[a](foo)",
        "[a](javascript:void)",
        "[a](ftp://example.com/f)",
        "[a](/relative)",
    ] {
        assert_eq!(
            from_markdown(input),
            doc(json!([paragraph(json!([text("a")]))])),
            "{input}"
        );
    }
}

#[test]
fn a_mail_address_is_a_link() {
    assert_eq!(
        from_markdown("[write](mailto:team@example.com)"),
        doc(json!([paragraph(json!([linked(
            "write",
            "mailto:team@example.com"
        )]))]))
    );
}

#[test]
fn a_link_without_text_shows_its_address() {
    assert_eq!(
        from_markdown("[](https://example.com)"),
        doc(json!([paragraph(json!([linked(
            "https://example.com",
            "https://example.com"
        )]))]))
    );
}

#[test]
fn an_address_inside_a_markdown_link_is_not_linked_twice() {
    assert_eq!(
        from_markdown("[see https://example.com/a](https://example.com/b)"),
        doc(json!([paragraph(json!([linked(
            "see https://example.com/a",
            "https://example.com/b"
        )]))]))
    );
}

#[test]
fn a_tight_bullet_list_holds_a_paragraph_per_item() {
    assert_eq!(
        from_markdown("- a\n- b"),
        doc(json!([{
            "type": "bulletList",
            "content": [
                list_item(json!([paragraph(json!([text("a")]))])),
                list_item(json!([paragraph(json!([text("b")]))])),
            ],
        }]))
    );
}

#[test]
fn a_loose_list_holds_a_paragraph_per_item_too() {
    assert_eq!(
        from_markdown("- a\n\n- b"),
        doc(json!([{
            "type": "bulletList",
            "content": [
                list_item(json!([paragraph(json!([text("a")]))])),
                list_item(json!([paragraph(json!([text("b")]))])),
            ],
        }]))
    );
}

#[test]
fn an_ordered_list_starting_at_one_has_no_attrs() {
    assert_eq!(
        from_markdown("1. a\n2. b"),
        doc(json!([{
            "type": "orderedList",
            "content": [
                list_item(json!([paragraph(json!([text("a")]))])),
                list_item(json!([paragraph(json!([text("b")]))])),
            ],
        }]))
    );
}

#[test]
fn an_ordered_list_keeps_its_start_number() {
    assert_eq!(
        from_markdown("3. a"),
        doc(json!([{
            "type": "orderedList",
            "attrs": { "order": 3 },
            "content": [list_item(json!([paragraph(json!([text("a")]))]))],
        }]))
    );
}

#[test]
fn a_nested_list_sits_in_its_item_after_the_paragraph() {
    assert_eq!(
        from_markdown("- a\n  1. b"),
        doc(json!([{
            "type": "bulletList",
            "content": [list_item(json!([
                paragraph(json!([text("a")])),
                {
                    "type": "orderedList",
                    "content": [list_item(json!([paragraph(json!([text("b")]))]))],
                },
            ]))],
        }]))
    );
}

#[test]
fn a_list_item_that_starts_with_a_nested_list_gets_an_empty_paragraph_first() {
    assert_eq!(
        from_markdown("- - x"),
        doc(json!([{
            "type": "bulletList",
            "content": [list_item(json!([
                paragraph(json!([])),
                {
                    "type": "bulletList",
                    "content": [list_item(json!([paragraph(json!([text("x")]))]))],
                },
            ]))],
        }]))
    );
}

#[test]
fn an_empty_list_item_holds_an_empty_paragraph() {
    assert_eq!(
        from_markdown("-"),
        doc(json!([{
            "type": "bulletList",
            "content": [list_item(json!([paragraph(json!([]))]))],
        }]))
    );
}

#[test]
fn a_heading_in_a_list_item_becomes_a_paragraph() {
    assert_eq!(
        from_markdown("- # Title"),
        doc(json!([{
            "type": "bulletList",
            "content": [list_item(json!([paragraph(json!([text("Title")]))]))],
        }]))
    );
}

#[test]
fn a_heading_keeps_its_level() {
    for (input, level) in [("# H", 1), ("### H", 3), ("###### H", 6)] {
        assert_eq!(
            from_markdown(input),
            doc(json!([{
                "type": "heading",
                "attrs": { "level": level },
                "content": [text("H")],
            }])),
            "{input}"
        );
    }
}

#[test]
fn a_quote_holds_its_paragraphs() {
    assert_eq!(
        from_markdown("> q"),
        doc(json!([{
            "type": "blockquote",
            "content": [paragraph(json!([text("q")]))],
        }]))
    );
}

#[test]
fn a_heading_in_a_quote_becomes_a_paragraph() {
    assert_eq!(
        from_markdown("> # Title"),
        doc(json!([{
            "type": "blockquote",
            "content": [paragraph(json!([text("Title")]))],
        }]))
    );
}

#[test]
fn a_quote_in_a_quote_is_flattened() {
    assert_eq!(
        from_markdown("> a\n>\n> > b"),
        doc(json!([{
            "type": "blockquote",
            "content": [
                paragraph(json!([text("a")])),
                paragraph(json!([text("b")])),
            ],
        }]))
    );
}

#[test]
fn a_fenced_code_block_keeps_its_language_and_text() {
    assert_eq!(
        from_markdown("```rust\nlet a = 1;\nlet b = 2;\n```"),
        doc(json!([{
            "type": "codeBlock",
            "attrs": { "language": "rust" },
            "content": [text("let a = 1;\nlet b = 2;")],
        }]))
    );
}

#[test]
fn a_fenced_code_block_without_a_language_has_no_attrs() {
    assert_eq!(
        from_markdown("```\nplain\n```"),
        doc(json!([{ "type": "codeBlock", "content": [text("plain")] }]))
    );
}

#[test]
fn an_empty_fenced_code_block_has_no_content() {
    assert_eq!(
        from_markdown("```\n```"),
        doc(json!([{ "type": "codeBlock", "content": [] }]))
    );
}

#[test]
fn nothing_inside_a_code_block_is_a_link_or_formatted() {
    assert_eq!(
        from_markdown("```\nsee https://example.com and **x**\n```"),
        doc(json!([{
            "type": "codeBlock",
            "content": [text("see https://example.com and **x**")],
        }]))
    );
}

#[test]
fn a_rule_between_paragraphs_is_a_rule() {
    assert_eq!(
        from_markdown("a\n\n---\n\nb"),
        doc(json!([
            paragraph(json!([text("a")])),
            { "type": "rule" },
            paragraph(json!([text("b")])),
        ]))
    );
}

#[test]
fn inline_html_stays_literal_text() {
    assert_eq!(
        from_markdown("a <b>x</b> c"),
        doc(json!([paragraph(json!([text("a <b>x</b> c")]))]))
    );
}

#[test]
fn an_escaped_marker_stays_literal() {
    assert_eq!(
        from_markdown("\\*not bold\\*"),
        doc(json!([paragraph(json!([text("*not bold*")]))]))
    );
}

#[test]
fn text_nodes_are_never_empty() {
    for input in [
        "a\n\nb",
        "**a** `b` [](https://example.com)",
        "- a\n- \n- - x",
        "> # H\n\n```\n```",
        "```rust\ncode\n```\n\n[a](foo) <i>x</i>",
        "one  \ntwo\n\nhttps://example.com.",
    ] {
        assert_no_empty_text(&from_markdown(input), input);
    }
}
