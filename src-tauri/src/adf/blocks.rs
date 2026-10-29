//! Block nodes: paragraphs, headings, lists, quotes, code, tables, panels, expands and media.

use serde_json::Value;

use super::{Media, children, escape, inline, one_of, wrap};

const PANELS: &[&str] = &[
    "info", "note", "warning", "success", "error", "tip", "custom",
];

/// Writes `node` when `kind` is a block node; `false` for any other kind.
pub(super) fn render(kind: &str, node: &Value, media: &Media, out: &mut String) -> bool {
    let attrs = &node["attrs"];
    match kind {
        "paragraph" => wrap("p", node, media, out),
        "heading" => {
            let level = attrs["level"].as_u64().unwrap_or(1).clamp(1, 6);
            wrap(&format!("h{level}"), node, media, out);
        }
        "bulletList" | "taskList" | "decisionList" => wrap("ul", node, media, out),
        "orderedList" => match attrs["order"].as_u64().filter(|&start| start != 1) {
            Some(start) => {
                out.push_str(&format!("<ol start=\"{start}\">"));
                children(node, media, out);
                out.push_str("</ol>");
            }
            None => wrap("ol", node, media, out),
        },
        "listItem" => wrap("li", node, media, out),
        "taskItem" => {
            out.push_str(if attrs["state"] == "DONE" {
                "<li>☑ "
            } else {
                "<li>☐ "
            });
            children(node, media, out);
            out.push_str("</li>");
        }
        "decisionItem" => {
            out.push_str("<li>◆ ");
            children(node, media, out);
            out.push_str("</li>");
        }
        "blockquote" => wrap("blockquote", node, media, out),
        "codeBlock" => code_block(node, out),
        "rule" => out.push_str("<hr>"),
        "table" => {
            out.push_str("<table><tbody>");
            children(node, media, out);
            out.push_str("</tbody></table>");
        }
        "tableRow" => wrap("tr", node, media, out),
        "tableHeader" => cell("th", node, media, out),
        "tableCell" => cell("td", node, media, out),
        "panel" => {
            let tone = one_of(attrs["panelType"].as_str(), PANELS, "info");
            out.push_str(&format!("<div class=\"adf-panel adf-panel--{tone}\">"));
            children(node, media, out);
            out.push_str("</div>");
        }
        "expand" | "nestedExpand" => {
            out.push_str("<div class=\"adf-expand\">");
            if let Some(title) = attrs["title"].as_str().filter(|t| !t.trim().is_empty()) {
                out.push_str("<p class=\"adf-expand__title\">");
                escape(title, out);
                out.push_str("</p>");
            }
            children(node, media, out);
            out.push_str("</div>");
        }
        "mediaSingle" | "mediaGroup" => {
            let single = (kind == "mediaSingle").then_some(attrs);
            out.push_str("<p>");
            for child in node["content"].as_array().into_iter().flatten() {
                match child["type"].as_str() {
                    Some("media") => super::media::render(child, single, media, out),
                    _ => super::node(child, media, out),
                }
            }
            out.push_str("</p>");
        }
        "blockCard" | "embedCard" => {
            out.push_str("<p>");
            inline::card(attrs, out);
            out.push_str("</p>");
        }
        _ => return false,
    }
    true
}

/// `<pre><code>`, with the language as the block's title.
fn code_block(node: &Value, out: &mut String) {
    match node["attrs"]["language"].as_str().filter(|l| !l.is_empty()) {
        Some(language) => {
            out.push_str("<pre title=\"");
            escape(language, out);
            out.push_str("\">");
        }
        None => out.push_str("<pre>"),
    }
    out.push_str("<code>");
    for child in node["content"].as_array().into_iter().flatten() {
        match child["type"].as_str() {
            Some("hardBreak") => out.push('\n'),
            _ => escape(child["text"].as_str().unwrap_or_default(), out),
        }
    }
    out.push_str("</code></pre>");
}

fn cell(tag: &str, node: &Value, media: &Media, out: &mut String) {
    out.push('<');
    out.push_str(tag);
    for span in ["colspan", "rowspan"] {
        if let Some(n) = node["attrs"][span].as_u64().filter(|&n| n > 1) {
            out.push_str(&format!(" {span}=\"{n}\""));
        }
    }
    out.push('>');
    children(node, media, out);
    out.push_str(&format!("</{tag}>"));
}
