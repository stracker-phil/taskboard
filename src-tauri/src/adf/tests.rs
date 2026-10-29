use serde_json::{Value, json};

use super::*;

fn doc(content: Value) -> Value {
    json!({ "type": "doc", "version": 1, "content": content })
}

fn html(content: Value) -> String {
    to_html(&doc(content)).expect("a document")
}

fn text(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn marked(text: &str, marks: Value) -> Value {
    json!({ "type": "text", "text": text, "marks": marks })
}

fn paragraph(content: Value) -> Value {
    json!({ "type": "paragraph", "content": content })
}

fn inline_html(node: Value) -> String {
    html(json!([paragraph(json!([node]))]))
}

fn marks_html(text: &str, marks: Value) -> String {
    inline_html(marked(text, marks))
}

#[test]
fn values_that_are_not_documents_render_nothing() {
    for (input, label) in [
        (json!({ "type": "paragraph", "content": [] }), "wrong type"),
        (json!({ "type": "doc" }), "missing content"),
        (
            json!({ "type": "doc", "content": "text" }),
            "string content",
        ),
        (json!({ "type": "doc", "content": {} }), "object content"),
        (json!("doc"), "string"),
        (Value::Null, "null"),
    ] {
        assert_eq!(to_html(&input), None, "{label}");
    }
}

#[test]
fn html_or_falls_back_when_the_json_is_invalid() {
    assert_eq!(
        html_or(Some("{not json"), Some("<p>jira</p>".into())),
        Some("<p>jira</p>".into())
    );
}

#[test]
fn html_or_falls_back_when_the_json_is_not_a_document() {
    assert_eq!(
        html_or(Some(r#"{"type":"paragraph"}"#), Some("<p>jira</p>".into())),
        Some("<p>jira</p>".into())
    );
}

#[test]
fn html_or_falls_back_when_there_is_no_adf() {
    assert_eq!(
        html_or(None, Some("<p>jira</p>".into())),
        Some("<p>jira</p>".into())
    );
    assert_eq!(html_or(None, None), None);
}

#[test]
fn html_or_prefers_a_valid_document_over_the_fallback() {
    let adf = doc(json!([paragraph(json!([text("Own")]))])).to_string();
    assert_eq!(
        html_or(Some(&adf), Some("<p>jira</p>".into())),
        Some("<p>Own</p>".into())
    );
}

#[test]
fn html_or_gives_no_content_for_an_empty_document_instead_of_the_fallback() {
    let adf = doc(json!([])).to_string();
    assert_eq!(html_or(Some(&adf), Some("<p>jira</p>".into())), None);
}

#[test]
fn a_paragraph_renders_as_p() {
    assert_eq!(
        html(json!([paragraph(json!([text("Hello")]))])),
        "<p>Hello</p>"
    );
}

#[test]
fn headings_render_their_level() {
    for level in 1..=6 {
        let node =
            json!({ "type": "heading", "attrs": { "level": level }, "content": [text("T")] });
        assert_eq!(
            html(json!([node])),
            format!("<h{level}>T</h{level}>"),
            "{level}"
        );
    }
}

#[test]
fn heading_levels_outside_the_range_are_clamped() {
    for (level, expected) in [(json!(0), "h1"), (json!(9), "h6"), (json!(null), "h1")] {
        let node =
            json!({ "type": "heading", "attrs": { "level": level }, "content": [text("T")] });
        assert_eq!(
            html(json!([node])),
            format!("<{expected}>T</{expected}>"),
            "{level}"
        );
    }
}

#[test]
fn a_bullet_list_renders_ul_with_li() {
    let list = json!({ "type": "bulletList", "content": [
        { "type": "listItem", "content": [paragraph(json!([text("a")]))] },
        { "type": "listItem", "content": [paragraph(json!([text("b")]))] },
    ] });
    assert_eq!(
        html(json!([list])),
        "<ul><li><p>a</p></li><li><p>b</p></li></ul>"
    );
}

#[test]
fn an_ordered_list_starts_at_its_order_unless_it_is_one() {
    let item = json!({ "type": "listItem", "content": [paragraph(json!([text("a")]))] });
    for (attrs, expected) in [
        (
            json!({ "order": 3 }),
            "<ol start=\"3\"><li><p>a</p></li></ol>",
        ),
        (json!({ "order": 1 }), "<ol><li><p>a</p></li></ol>"),
        (json!({}), "<ol><li><p>a</p></li></ol>"),
    ] {
        let list = json!({ "type": "orderedList", "attrs": attrs, "content": [item] });
        assert_eq!(html(json!([list])), expected, "{attrs}");
    }
}

#[test]
fn task_items_show_a_checked_box_only_when_done() {
    let list = json!({ "type": "taskList", "content": [
        { "type": "taskItem", "attrs": { "state": "DONE" }, "content": [text("shipped")] },
        { "type": "taskItem", "attrs": { "state": "TODO" }, "content": [text("open")] },
        { "type": "taskItem", "content": [text("bare")] },
    ] });
    assert_eq!(
        html(json!([list])),
        "<ul><li>☑ shipped</li><li>☐ open</li><li>☐ bare</li></ul>"
    );
}

#[test]
fn decision_items_are_marked_with_a_diamond() {
    let list = json!({ "type": "decisionList", "content": [
        { "type": "decisionItem", "content": [text("Use SQLite")] },
    ] });
    assert_eq!(html(json!([list])), "<ul><li>◆ Use SQLite</li></ul>");
}

#[test]
fn a_blockquote_wraps_its_blocks() {
    let quote = json!({ "type": "blockquote", "content": [paragraph(json!([text("Quoted")]))] });
    assert_eq!(
        html(json!([quote])),
        "<blockquote><p>Quoted</p></blockquote>"
    );
}

#[test]
fn a_code_block_titles_its_language_and_keeps_newlines() {
    let block = json!({
        "type": "codeBlock",
        "attrs": { "language": "php" },
        "content": [text("echo 1;\nreturn $a < $b;")],
    });
    assert_eq!(
        html(json!([block])),
        "<pre title=\"php\"><code>echo 1;\nreturn $a &lt; $b;</code></pre>"
    );
}

#[test]
fn a_code_block_without_a_language_has_no_title() {
    for attrs in [json!({}), json!({ "language": "" })] {
        let block = json!({ "type": "codeBlock", "attrs": attrs, "content": [text("x")] });
        assert_eq!(html(json!([block])), "<pre><code>x</code></pre>", "{attrs}");
    }
}

#[test]
fn a_code_block_escapes_markup_and_never_writes_a_tag() {
    let block = json!({ "type": "codeBlock", "content": [text("<script>alert(1)</script>")] });
    assert_eq!(
        html(json!([block])),
        "<pre><code>&lt;script&gt;alert(1)&lt;/script&gt;</code></pre>"
    );
}

#[test]
fn a_code_block_turns_hard_breaks_into_newlines() {
    let block = json!({ "type": "codeBlock", "content": [
        text("a"), { "type": "hardBreak" }, text("b"),
    ] });
    assert_eq!(html(json!([block])), "<pre><code>a\nb</code></pre>");
}

#[test]
fn a_code_block_language_is_attribute_escaped() {
    let block = json!({
        "type": "codeBlock",
        "attrs": { "language": "x\" onclick=\"y" },
        "content": [text("c")],
    });
    assert_eq!(
        html(json!([block])),
        "<pre title=\"x&quot; onclick=&quot;y\"><code>c</code></pre>"
    );
}

#[test]
fn a_rule_renders_hr_and_a_hard_break_renders_br() {
    assert_eq!(
        html(json!([
            { "type": "rule" },
            paragraph(json!([text("a"), { "type": "hardBreak" }, text("b")])),
        ])),
        "<hr><p>a<br>b</p>"
    );
}

#[test]
fn a_table_renders_headers_and_cells() {
    let table = json!({ "type": "table", "content": [
        { "type": "tableRow", "content": [
            { "type": "tableHeader", "content": [paragraph(json!([text("Name")]))] },
            { "type": "tableHeader", "content": [paragraph(json!([text("Qty")]))] },
        ] },
        { "type": "tableRow", "content": [
            { "type": "tableCell", "content": [paragraph(json!([text("Pen")]))] },
            { "type": "tableCell", "content": [paragraph(json!([text("2")]))] },
        ] },
    ] });
    assert_eq!(
        html(json!([table])),
        "<table><tbody>\
         <tr><th><p>Name</p></th><th><p>Qty</p></th></tr>\
         <tr><td><p>Pen</p></td><td><p>2</p></td></tr>\
         </tbody></table>"
    );
}

#[test]
fn cell_spans_are_written_only_above_one() {
    let table = json!({ "type": "table", "content": [{ "type": "tableRow", "content": [
        { "type": "tableCell", "attrs": { "colspan": 2, "rowspan": 3 }, "content": [text("a")] },
        { "type": "tableCell", "attrs": { "colspan": 1, "rowspan": 1 }, "content": [text("b")] },
        { "type": "tableHeader", "attrs": { "colspan": 2 }, "content": [text("c")] },
    ] }] });
    assert_eq!(
        html(json!([table])),
        "<table><tbody><tr>\
         <td colspan=\"2\" rowspan=\"3\">a</td>\
         <td>b</td>\
         <th colspan=\"2\">c</th>\
         </tr></tbody></table>"
    );
}

#[test]
fn a_panel_renders_its_type_as_a_class() {
    let panel = json!({
        "type": "panel",
        "attrs": { "panelType": "warning" },
        "content": [paragraph(json!([text("Careful")]))],
    });
    assert_eq!(
        html(json!([panel])),
        "<div class=\"adf-panel adf-panel--warning\"><p>Careful</p></div>"
    );
}

#[test]
fn an_unknown_panel_type_falls_back_to_info() {
    for panel_type in [
        json!("evil\" onclick=\"x"),
        json!("sparkly"),
        json!(null),
        json!(7),
    ] {
        let panel = json!({
            "type": "panel",
            "attrs": { "panelType": panel_type },
            "content": [text("x")],
        });
        assert_eq!(
            html(json!([panel])),
            "<div class=\"adf-panel adf-panel--info\">x</div>",
            "{panel_type}"
        );
    }
}

#[test]
fn expand_and_nested_expand_render_an_escaped_title() {
    for kind in ["expand", "nestedExpand"] {
        let node = json!({
            "type": kind,
            "attrs": { "title": "Steps <1>" },
            "content": [paragraph(json!([text("Body")]))],
        });
        assert_eq!(
            html(json!([node])),
            "<div class=\"adf-expand\"><p class=\"adf-expand__title\">Steps &lt;1&gt;</p>\
             <p>Body</p></div>",
            "{kind}"
        );
    }
}

#[test]
fn an_expand_without_a_title_has_no_title_line() {
    for attrs in [json!({}), json!({ "title": "  " })] {
        let node = json!({
            "type": "expand",
            "attrs": attrs,
            "content": [paragraph(json!([text("Body")]))],
        });
        assert_eq!(
            html(json!([node])),
            "<div class=\"adf-expand\"><p>Body</p></div>",
            "{attrs}"
        );
    }
}

#[test]
fn a_media_single_renders_a_named_attachment_chip() {
    let node = json!({ "type": "mediaSingle", "content": [
        { "type": "media", "attrs": { "type": "file", "id": "abc", "alt": "wiring.png" } },
    ] });
    assert_eq!(
        html(json!([node])),
        "<p><span class=\"jira-chip\">Attachment · wiring.png</span> </p>"
    );
}

#[test]
fn a_media_name_comes_from_alt_then_name_then_file_name() {
    for (attrs, expected) in [
        (
            json!({ "alt": "a.png", "name": "n.png", "__fileName": "f.png" }),
            "a.png",
        ),
        (json!({ "name": "n.png", "__fileName": "f.png" }), "n.png"),
        (json!({ "__fileName": "f.png" }), "f.png"),
        (json!({ "alt": "  ", "name": "n.png" }), "n.png"),
    ] {
        let node =
            json!({ "type": "mediaSingle", "content": [{ "type": "media", "attrs": attrs }] });
        assert_eq!(
            html(json!([node])),
            format!("<p><span class=\"jira-chip\">Attachment · {expected}</span> </p>"),
            "{attrs}"
        );
    }
}

#[test]
fn a_media_without_a_name_is_just_an_attachment() {
    let node = json!({ "type": "mediaSingle", "content": [{ "type": "media", "attrs": {} }] });
    assert_eq!(
        html(json!([node])),
        "<p><span class=\"jira-chip\">Attachment</span> </p>"
    );
}

#[test]
fn a_media_group_renders_one_chip_per_media() {
    let node = json!({ "type": "mediaGroup", "content": [
        { "type": "media", "attrs": { "name": "a.pdf" } },
        { "type": "media", "attrs": { "name": "b.pdf" } },
    ] });
    assert_eq!(
        html(json!([node])),
        "<p><span class=\"jira-chip\">Attachment · a.pdf</span> \
         <span class=\"jira-chip\">Attachment · b.pdf</span> </p>"
    );
}

#[test]
fn an_inline_media_renders_a_chip_inside_the_paragraph() {
    let node = json!({ "type": "mediaInline", "attrs": { "name": "log.txt" } });
    assert_eq!(
        inline_html(node),
        "<p><span class=\"jira-chip\">Attachment · log.txt</span> </p>"
    );
}

#[test]
fn media_never_reference_remote_content() {
    let node = json!({ "type": "mediaSingle", "content": [{ "type": "media", "attrs": {
        "name": "a.png",
        "url": "https://example.com/a.png",
        "src": "https://example.com/a.png",
    } }] });
    let out = html(json!([node]));
    assert!(!out.contains("src"), "{out}");
    assert!(!out.contains("<img"), "{out}");
    assert!(!out.contains("example.com"), "{out}");
}

#[test]
fn simple_marks_wrap_the_text() {
    for (mark, expected) in [
        ("strong", "<p><strong>x</strong></p>"),
        ("em", "<p><em>x</em></p>"),
        ("underline", "<p><u>x</u></p>"),
        ("strike", "<p><s>x</s></p>"),
        ("code", "<p><code>x</code></p>"),
    ] {
        assert_eq!(
            marks_html("x", json!([{ "type": mark }])),
            expected,
            "{mark}"
        );
    }
}

#[test]
fn subscript_and_superscript_marks_render_sub_and_sup() {
    for (kind, expected) in [
        ("sub", "<p><sub>2</sub></p>"),
        ("sup", "<p><sup>2</sup></p>"),
    ] {
        assert_eq!(
            marks_html(
                "2",
                json!([{ "type": "subsup", "attrs": { "type": kind } }])
            ),
            expected,
            "{kind}"
        );
    }
}

#[test]
fn a_text_color_is_ignored_but_the_text_stays() {
    assert_eq!(
        marks_html(
            "red",
            json!([{ "type": "textColor", "attrs": { "color": "#ff0000" } }])
        ),
        "<p>red</p>"
    );
}

#[test]
fn combined_marks_nest_with_the_link_outermost_and_code_innermost() {
    let marks = json!([
        { "type": "code" },
        { "type": "strong" },
        { "type": "link", "attrs": { "href": "https://example.com/x" } },
        { "type": "em" },
    ]);
    assert_eq!(
        marks_html("x", marks),
        "<p><a href=\"https://example.com/x\"><strong><em><code>x</code></em></strong></a></p>"
    );
}

#[test]
fn a_https_link_keeps_its_address() {
    assert_eq!(
        marks_html(
            "docs",
            json!([{ "type": "link", "attrs": { "href": "https://example.com/x" } }])
        ),
        "<p><a href=\"https://example.com/x\">docs</a></p>"
    );
}

#[test]
fn a_site_relative_link_keeps_its_path() {
    assert_eq!(
        marks_html(
            "NW-1",
            json!([{ "type": "link", "attrs": { "href": "/browse/NW-1" } }])
        ),
        "<p><a href=\"/browse/NW-1\">NW-1</a></p>"
    );
}

#[test]
fn links_the_page_may_not_open_lose_their_href_but_keep_the_text() {
    for href in [
        "http://example.com/x",
        "javascript:alert(1)",
        "//evil.example.com/x",
        "mailto:dev@example.com",
        "",
    ] {
        assert_eq!(
            marks_html("go", json!([{ "type": "link", "attrs": { "href": href } }])),
            "<p><a>go</a></p>",
            "{href}"
        );
    }
}

#[test]
fn a_link_without_an_href_renders_an_anchor_without_it() {
    assert_eq!(
        marks_html("go", json!([{ "type": "link" }])),
        "<p><a>go</a></p>"
    );
}

#[test]
fn a_link_href_is_attribute_escaped() {
    assert_eq!(
        marks_html(
            "go",
            json!([{ "type": "link", "attrs": { "href": "https://example.com/\" onclick=\"x" } }])
        ),
        "<p><a href=\"https://example.com/&quot; onclick=&quot;x\">go</a></p>"
    );
}

#[test]
fn a_mention_shows_the_name_with_one_at_sign() {
    for name in ["Ada Example", "@Ada Example"] {
        assert_eq!(
            inline_html(json!({ "type": "mention", "attrs": { "id": "1", "text": name } })),
            "<p><span class=\"adf-mention\">@Ada Example</span></p>",
            "{name}"
        );
    }
}

#[test]
fn a_mention_name_is_escaped() {
    assert_eq!(
        inline_html(json!({ "type": "mention", "attrs": { "text": "<script>alert(1)</script>" } })),
        "<p><span class=\"adf-mention\">@&lt;script&gt;alert(1)&lt;/script&gt;</span></p>"
    );
}

#[test]
fn an_emoji_shows_its_text_else_its_short_name() {
    for (attrs, expected) in [
        (json!({ "shortName": ":smile:", "text": "😄" }), "😄"),
        (json!({ "shortName": ":smile:" }), ":smile:"),
    ] {
        assert_eq!(
            inline_html(json!({ "type": "emoji", "attrs": attrs })),
            format!("<p>{expected}</p>"),
            "{attrs}"
        );
    }
}

#[test]
fn a_date_shows_the_timestamp_as_a_calendar_day() {
    for timestamp in [json!("1793361600000"), json!(1793361600000_i64)] {
        assert_eq!(
            inline_html(json!({ "type": "date", "attrs": { "timestamp": timestamp } })),
            "<p>Oct 30, 2026</p>",
            "{timestamp}"
        );
    }
}

#[test]
fn a_date_without_a_usable_timestamp_renders_nothing() {
    for timestamp in [json!("soon"), json!(null)] {
        assert_eq!(
            inline_html(json!({ "type": "date", "attrs": { "timestamp": timestamp } })),
            "<p></p>",
            "{timestamp}"
        );
    }
}

#[test]
fn a_status_renders_a_lozenge_in_its_color() {
    assert_eq!(
        inline_html(json!({ "type": "status", "attrs": { "text": "DONE", "color": "green" } })),
        "<p><span class=\"adf-status adf-status--green\">DONE</span></p>"
    );
}

#[test]
fn a_status_with_an_unknown_color_is_neutral() {
    for color in [json!("magenta"), json!("x\" onclick=\"y"), json!(null)] {
        assert_eq!(
            inline_html(json!({ "type": "status", "attrs": { "text": "OPEN", "color": color } })),
            "<p><span class=\"adf-status adf-status--neutral\">OPEN</span></p>",
            "{color}"
        );
    }
}

#[test]
fn a_status_text_is_escaped() {
    assert_eq!(
        inline_html(json!({ "type": "status", "attrs": {
            "text": "<script>alert(1)</script>", "color": "red",
        } })),
        "<p><span class=\"adf-status adf-status--red\">&lt;script&gt;alert(1)&lt;/script&gt;</span></p>"
    );
}

#[test]
fn cards_with_a_https_url_render_a_link_showing_the_url() {
    let url = "https://example.com/spec";
    let link = format!("<a href=\"{url}\">{url}</a>");
    assert_eq!(
        inline_html(json!({ "type": "inlineCard", "attrs": { "url": url } })),
        format!("<p>{link}</p>")
    );
    assert_eq!(
        html(json!([{ "type": "blockCard", "attrs": { "url": url } }])),
        format!("<p>{link}</p>")
    );
}

#[test]
fn cards_with_another_url_render_plain_escaped_text() {
    for (url, expected) in [
        (
            "http://example.com/a?x=1&y=2",
            "http://example.com/a?x=1&amp;y=2",
        ),
        ("javascript:alert(\"1\")", "javascript:alert(&quot;1&quot;)"),
    ] {
        assert_eq!(
            inline_html(json!({ "type": "inlineCard", "attrs": { "url": url } })),
            format!("<p>{expected}</p>"),
            "{url}"
        );
    }
}

#[test]
fn markup_in_text_and_headings_is_escaped() {
    assert_eq!(
        html(json!([paragraph(json!([text(
            "<script>alert(1)</script>"
        )]))])),
        "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>"
    );
    let heading = json!({
        "type": "heading",
        "attrs": { "level": 2 },
        "content": [text("<script>alert(1)</script>")],
    });
    assert_eq!(
        html(json!([heading])),
        "<h2>&lt;script&gt;alert(1)&lt;/script&gt;</h2>"
    );
}

#[test]
fn quotes_apostrophes_and_ampersands_in_text_are_escaped() {
    assert_eq!(
        html(json!([paragraph(json!([text("Tom & \"Jerry\" it's")]))])),
        "<p>Tom &amp; &quot;Jerry&quot; it&#39;s</p>"
    );
}

#[test]
fn an_arrow_chain_in_plain_text_is_not_struck_through() {
    let out = html(json!([paragraph(json!([text(
        "Shop()->cart->get_totals()"
    )]))]));
    assert_eq!(out, "<p>Shop()-&gt;cart-&gt;get_totals()</p>");
    assert!(!out.contains("<s>") && !out.contains("<del>"), "{out}");
}

#[test]
fn an_arrow_chain_in_a_code_mark_stays_literal_inside_code() {
    let out = marks_html("Shop()->cart->get_totals()", json!([{ "type": "code" }]));
    assert_eq!(out, "<p><code>Shop()-&gt;cart-&gt;get_totals()</code></p>");
    assert!(!out.contains("<s>") && !out.contains("<del>"), "{out}");
}

#[test]
fn double_dash_flags_stay_literal() {
    assert_eq!(
        html(json!([paragraph(json!([text("run with --no-cache now")]))])),
        "<p>run with --no-cache now</p>"
    );
    assert_eq!(
        marks_html("--no-cache", json!([{ "type": "code" }])),
        "<p><code>--no-cache</code></p>"
    );
}

#[test]
fn an_unknown_node_with_content_renders_its_children() {
    let unknown = json!({ "type": "layoutSection", "content": [
        { "type": "layoutColumn", "content": [paragraph(json!([text("Left")]))] },
        { "type": "layoutColumn", "content": [paragraph(json!([text("Right")]))] },
    ] });
    assert_eq!(html(json!([unknown])), "<p>Left</p><p>Right</p>");
}

#[test]
fn an_unknown_node_with_text_renders_the_text_with_its_marks() {
    let unknown =
        json!({ "type": "mystery", "text": "<b>hi</b>", "marks": [{ "type": "strong" }] });
    assert_eq!(
        html(json!([paragraph(json!([unknown]))])),
        "<p><strong>&lt;b&gt;hi&lt;/b&gt;</strong></p>"
    );
}

#[test]
fn nodes_that_are_not_objects_or_have_no_type_are_skipped() {
    let content = json!([
        "stray",
        42,
        null,
        { "text": "no type" },
        { "type": 7, "text": "numeric type" },
        paragraph(json!([text("kept")])),
    ]);
    assert_eq!(html(content), "<p>kept</p>");
}

#[test]
fn content_that_is_not_an_array_inside_a_node_is_empty() {
    for content in [json!("oops"), json!({}), json!(null), json!(3)] {
        let out = html(json!([
            { "type": "paragraph", "content": content },
            { "type": "mystery", "content": content },
        ]));
        assert_eq!(out, "<p></p>", "{content}");
    }
}
